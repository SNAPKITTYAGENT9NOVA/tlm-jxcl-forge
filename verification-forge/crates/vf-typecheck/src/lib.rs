// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Bidirectional type-checking used during elaboration.
//!
//! ## Trust status: this crate is UNTRUSTED
//!
//! Per `docs/TRUST_MODEL.md`, `vf-typecheck` is an *evidence
//! generator*, not part of the trusted computing base. It exists to
//! give `vf-elab` fast, helpful type errors while turning surface
//! syntax into a candidate `vf-core::Term`. Whatever type this crate
//! assigns a term is a claim, not a fact: `vf-kernel` (not yet built)
//! independently re-derives the type of every term it is asked to
//! accept, using the same `vf-reducer` primitives but trusting nothing
//! this crate concluded. A bug in this crate can produce a bad
//! *candidate* proof term; it can never, by itself, make the kernel
//! accept an invalid one.
//!
//! ## Algorithm
//!
//! A minimal bidirectional checker over `vf-core::Term`:
//! - [`infer`] computes a term's type outright.
//! - [`check`] checks a term against an expected type; only
//!   [`vf_core::Term::Lam`] gets genuine check-mode treatment (its
//!   parameter type is checked against the expected `Pi`'s domain
//!   instead of trusting the annotation alone), everything else infers
//!   and compares via [`vf_reducer::def_eq`]. A fuller bidirectional
//!   algorithm (e.g. propagating expected types into `App` arguments)
//!   is possible but not needed for a checker whose conclusions are
//!   always re-derived by the kernel anyway.
//!
//! Local variables are tracked in a [`Context`] keyed by the
//! [`vf_core::Symbol`] a binder's body was opened with (see
//! `vf_core::Interner::fresh`), matching `vf-core`'s locally-nameless
//! representation: this crate never manipulates a raw
//! [`vf_core::Term::BoundVar`] itself, only ever variables that have
//! already been opened into a fresh free variable.
#![forbid(unsafe_code)]

use std::fmt;
use vf_core::{ArenaError, Interner, Sort, Symbol, Term, TermArena, TermId};
use vf_reducer::{DeltaContext, ReduceError};

/// The typing context: what type each in-scope local (opened) free
/// variable has.
#[derive(Debug, Clone, Default)]
pub struct Context {
    bindings: Vec<(Symbol, TermId)>,
}

impl Context {
    pub fn new() -> Self {
        Context::default()
    }

    /// A new context with `sym : ty` added on top of `self`, shadowing
    /// any existing binding for `sym`.
    pub fn extended(&self, sym: Symbol, ty: TermId) -> Self {
        let mut bindings = self.bindings.clone();
        bindings.push((sym, ty));
        Context { bindings }
    }

    pub fn lookup(&self, sym: Symbol) -> Option<TermId> {
        self.bindings
            .iter()
            .rev()
            .find(|(s, _)| *s == sym)
            .map(|(_, t)| *t)
    }
}

/// What the global environment (owned by `vf-axioms`/`vf-kernel`,
/// neither of which exists yet) must be able to answer for
/// type-checking to proceed: the type of every declared constant, plus
/// (via [`DeltaContext`]) how to unfold the ones that have a
/// definition.
pub trait GlobalEnv: DeltaContext {
    fn type_of_const(&self, name: Symbol) -> Option<TermId>;
}

/// A [`GlobalEnv`] with no declared constants at all. Useful for
/// checking closed terms that only use `vf-core`'s built-in formers
/// (`Sort`/`Pi`/`Lam`/`App`/`Let`/`Eq`).
pub struct NoGlobals;
impl DeltaContext for NoGlobals {
    fn unfold(&self, _name: Symbol) -> Option<TermId> {
        None
    }
}
impl GlobalEnv for NoGlobals {
    fn type_of_const(&self, _name: Symbol) -> Option<TermId> {
        None
    }
}

/// Everything that can make a term fail to type-check. Never a panic:
/// every case here is a reported judgment, not a `Result` this crate
/// ever discards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    Arena(ArenaError),
    Reduce(ReduceError),
    /// A [`Term::BoundVar`] was seen where a properly-opened
    /// (locally-nameless) term was expected. This indicates the input
    /// term is not well-scoped -- never a valid proof/definition body,
    /// regardless of what it claims to prove.
    UnexpectedLooseBoundVar,
    UnboundVariable(Symbol),
    UnknownConstant(Symbol),
    NotAFunction {
        head_type: TermId,
    },
    NotASort {
        got: TermId,
    },
    TypeMismatch {
        expected: TermId,
        found: TermId,
    },
}

impl From<ArenaError> for TypeError {
    fn from(e: ArenaError) -> Self {
        TypeError::Arena(e)
    }
}
impl From<ReduceError> for TypeError {
    fn from(e: ReduceError) -> Self {
        TypeError::Reduce(e)
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Arena(e) => write!(f, "{e}"),
            TypeError::Reduce(e) => write!(f, "{e}"),
            TypeError::UnexpectedLooseBoundVar => {
                write!(
                    f,
                    "encountered a bound variable outside any binder (not well-scoped)"
                )
            }
            TypeError::UnboundVariable(_) => write!(f, "unbound local variable"),
            TypeError::UnknownConstant(_) => write!(f, "reference to an undeclared constant"),
            TypeError::NotAFunction { .. } => {
                write!(f, "applied a term whose type is not a Pi (function) type")
            }
            TypeError::NotASort { .. } => {
                write!(f, "expected a type (a term of sort Prop or Type(n))")
            }
            TypeError::TypeMismatch { .. } => write!(f, "type mismatch"),
        }
    }
}
impl std::error::Error for TypeError {}

/// The sort of `Pi (_ : A), B` given `A : domain_sort` and
/// `B : codomain_sort`, under the (impredicative-Prop, predicative
/// cumulative Type) rule standard to the Calculus of Constructions:
/// a `Prop`-valued codomain makes the whole Pi type a `Prop`
/// regardless of the domain's sort; otherwise the Pi lives at the join
/// of both sorts' levels.
fn sort_of_pi(domain_sort: Sort, codomain_sort: Sort) -> Sort {
    match codomain_sort {
        Sort::Prop => Sort::Prop,
        Sort::Type(m) => {
            let n = match domain_sort {
                Sort::Prop => 0,
                Sort::Type(n) => n,
            };
            Sort::Type(n.max(m))
        }
    }
}

/// Reduce `term` to whnf and require the result to be a [`Term::Sort`].
fn as_sort<E: GlobalEnv>(
    arena: &mut TermArena,
    env: &E,
    term: TermId,
    fuel: &mut u64,
) -> Result<Sort, TypeError> {
    let w = vf_reducer::whnf(arena, env, term, fuel)?;
    match arena.get(w)? {
        Term::Sort(s) => Ok(*s),
        _ => Err(TypeError::NotASort { got: w }),
    }
}

/// Reduce `term` to whnf and require the result to be a [`Term::Pi`],
/// returning its `(domain, codomain)`.
fn as_pi<E: GlobalEnv>(
    arena: &mut TermArena,
    env: &E,
    term: TermId,
    fuel: &mut u64,
) -> Result<(TermId, TermId), TypeError> {
    let w = vf_reducer::whnf(arena, env, term, fuel)?;
    match arena.get(w)?.clone() {
        Term::Pi {
            domain, codomain, ..
        } => Ok((domain, codomain)),
        _ => Err(TypeError::NotAFunction { head_type: w }),
    }
}

/// Require `term` to be a well-formed *type*: infer `term`'s own type
/// and require that it reduces to a [`Term::Sort`], returning which
/// one. This is the Pi/Lam/Let/Eq formation rules' shared "is this a
/// type" check -- note it is `infer(term)` that must be a sort, not
/// `term` itself (a domain type like a future `Const("Nat")` is not
/// literally a [`Term::Sort`], but its *type* `Type 0` is).
fn sort_of<E: GlobalEnv>(
    arena: &mut TermArena,
    interner: &mut Interner,
    env: &E,
    ctx: &Context,
    term: TermId,
    fuel: &mut u64,
) -> Result<Sort, TypeError> {
    let ty = infer(arena, interner, env, ctx, term, fuel)?;
    as_sort(arena, env, ty, fuel)
}

/// Infer `term`'s type under `ctx`/`env`.
pub fn infer<E: GlobalEnv>(
    arena: &mut TermArena,
    interner: &mut Interner,
    env: &E,
    ctx: &Context,
    term: TermId,
    fuel: &mut u64,
) -> Result<TermId, TypeError> {
    let t = arena.get(term)?.clone();
    match t {
        Term::Sort(s) => Ok(arena.sort(s.classifier())),
        Term::BoundVar(_) => Err(TypeError::UnexpectedLooseBoundVar),
        Term::FreeVar(sym) => ctx.lookup(sym).ok_or(TypeError::UnboundVariable(sym)),
        Term::Const(sym) => env
            .type_of_const(sym)
            .ok_or(TypeError::UnknownConstant(sym)),
        Term::App(f, a) => {
            let f_ty = infer(arena, interner, env, ctx, f, fuel)?;
            let (domain, codomain) = as_pi(arena, env, f_ty, fuel)?;
            check(arena, interner, env, ctx, a, domain, fuel)?;
            Ok(arena.open_at(codomain, 0, a)?)
        }
        Term::Lam { binder, ty, body } => {
            // The annotation itself must be a type.
            sort_of(arena, interner, env, ctx, ty, fuel)?;
            let sym = interner.fresh();
            let free = arena.free_var(sym);
            let opened_body = arena.open_at(body, 0, free)?;
            let inner_ctx = ctx.extended(sym, ty);
            let body_ty = infer(arena, interner, env, &inner_ctx, opened_body, fuel)?;
            let closed_body_ty = arena.close_at(body_ty, 0, sym)?;
            Ok(arena.pi(binder, ty, closed_body_ty))
        }
        Term::Pi {
            domain, codomain, ..
        } => {
            let domain_sort = sort_of(arena, interner, env, ctx, domain, fuel)?;
            let sym = interner.fresh();
            let free = arena.free_var(sym);
            let opened_codomain = arena.open_at(codomain, 0, free)?;
            let inner_ctx = ctx.extended(sym, domain);
            let codomain_sort = sort_of(arena, interner, env, &inner_ctx, opened_codomain, fuel)?;
            Ok(arena.sort(sort_of_pi(domain_sort, codomain_sort)))
        }
        Term::Let {
            ty, value, body, ..
        } => {
            sort_of(arena, interner, env, ctx, ty, fuel)?;
            check(arena, interner, env, ctx, value, ty, fuel)?;
            let sym = interner.fresh();
            let free = arena.free_var(sym);
            let opened_body = arena.open_at(body, 0, free)?;
            let inner_ctx = ctx.extended(sym, ty);
            let body_ty = infer(arena, interner, env, &inner_ctx, opened_body, fuel)?;
            // The result type must not mention the local `x` once it
            // goes out of scope: substitute `value` for it directly.
            let closed_body_ty = arena.close_at(body_ty, 0, sym)?;
            Ok(arena.open_at(closed_body_ty, 0, value)?)
        }
        Term::Eq { ty, lhs, rhs } => {
            sort_of(arena, interner, env, ctx, ty, fuel)?;
            check(arena, interner, env, ctx, lhs, ty, fuel)?;
            check(arena, interner, env, ctx, rhs, ty, fuel)?;
            Ok(arena.sort(Sort::Prop))
        }
    }
}

/// Check `term` against `expected_ty` under `ctx`/`env`.
pub fn check<E: GlobalEnv>(
    arena: &mut TermArena,
    interner: &mut Interner,
    env: &E,
    ctx: &Context,
    term: TermId,
    expected_ty: TermId,
    fuel: &mut u64,
) -> Result<(), TypeError> {
    let t = arena.get(term)?.clone();
    if let Term::Lam { ty, body, .. } = t {
        let (domain, codomain) = as_pi(arena, env, expected_ty, fuel)?;
        if !vf_reducer::def_eq(arena, env, ty, domain, fuel)? {
            return Err(TypeError::TypeMismatch {
                expected: domain,
                found: ty,
            });
        }
        let sym = interner.fresh();
        let free = arena.free_var(sym);
        let opened_body = arena.open_at(body, 0, free)?;
        let opened_codomain = arena.open_at(codomain, 0, free)?;
        let inner_ctx = ctx.extended(sym, domain);
        return check(
            arena,
            interner,
            env,
            &inner_ctx,
            opened_body,
            opened_codomain,
            fuel,
        );
    }

    let inferred = infer(arena, interner, env, ctx, term, fuel)?;
    if vf_reducer::def_eq(arena, env, inferred, expected_ty, fuel)? {
        Ok(())
    } else {
        Err(TypeError::TypeMismatch {
            expected: expected_ty,
            found: inferred,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_core::Binder;
    use vf_reducer::DEFAULT_FUEL;

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    /// `Type 0 -> Type 0`, i.e. the type of a (non-dependent) function
    /// from `Type 0` to `Type 0`.
    fn nondep_pi(
        arena: &mut TermArena,
        interner: &mut Interner,
        domain: TermId,
        codomain: TermId,
    ) -> TermId {
        let dummy = Binder(interner.intern("_"));
        // codomain doesn't mention the bound var, so shifting it in is a no-op.
        let shifted = arena.shift(codomain, 0, 1).unwrap();
        arena.pi(dummy, domain, shifted)
    }

    #[test]
    fn sort_prop_has_type_type_zero() {
        let (mut arena, mut interner) = setup();
        let prop = arena.sort(Sort::Prop);
        let mut fuel = DEFAULT_FUEL;
        let ty = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            prop,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(ty, arena.sort(Sort::Type(0)));
    }

    #[test]
    fn identity_function_infers_a_non_dependent_pi_type() {
        // fun (x : Type 0) => x  :  Pi (_ : Type 0), Type 0
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let body = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, body);

        let mut fuel = DEFAULT_FUEL;
        let inferred = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            id_fn,
            &mut fuel,
        )
        .unwrap();
        let expected = nondep_pi(&mut arena, &mut interner, ty0, ty0);
        assert_eq!(inferred, expected);
    }

    #[test]
    fn applying_the_identity_function_infers_the_argument_type() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty1, x0);
        let applied = arena.app(id_fn, ty0);

        let mut fuel = DEFAULT_FUEL;
        let inferred = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            applied,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(inferred, ty1);
    }

    #[test]
    fn checking_a_lambda_against_a_pi_succeeds_when_domains_match() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);
        let expected = nondep_pi(&mut arena, &mut interner, ty0, ty0);

        let mut fuel = DEFAULT_FUEL;
        check(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            id_fn,
            expected,
            &mut fuel,
        )
        .unwrap();
    }

    #[test]
    fn checking_a_lambda_against_a_mismatched_domain_fails() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);
        // Expects a function FROM Type 1, but id_fn's parameter is Type 0.
        let expected = nondep_pi(&mut arena, &mut interner, ty1, ty1);

        let mut fuel = DEFAULT_FUEL;
        let err = check(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            id_fn,
            expected,
            &mut fuel,
        )
        .unwrap_err();
        assert!(matches!(err, TypeError::TypeMismatch { .. }));
    }

    #[test]
    fn pi_type_itself_is_well_sorted() {
        // Pi (_ : Type 0), Type 0  :  Type 1  (join of level 0 and level 0's classifier)
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let pi = nondep_pi(&mut arena, &mut interner, ty0, ty0);
        let mut fuel = DEFAULT_FUEL;
        let sort = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            pi,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(sort, arena.sort(Sort::Type(1)));
    }

    #[test]
    fn a_prop_valued_pi_is_impredicatively_in_prop() {
        // Pi (_ : Type 5), Eq(Type 0, Prop, Prop)  :  Prop, regardless
        // of the domain's level (impredicativity). The codomain must
        // itself be a term OF sort Prop -- unlike the sort literal
        // `Prop` (which denotes the universe of propositions and so is
        // itself of sort `Type 0`), `Eq(...)` genuinely infers to Prop.
        let (mut arena, mut interner) = setup();
        let ty5 = arena.sort(Sort::Type(5));
        let ty0 = arena.sort(Sort::Type(0));
        let prop = arena.sort(Sort::Prop);
        let codomain = arena.eq(ty0, prop, prop); // Prop : Type 0, so this is well-typed.
        let pi = nondep_pi(&mut arena, &mut interner, ty5, codomain);
        let mut fuel = DEFAULT_FUEL;
        let sort = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            pi,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(sort, arena.sort(Sort::Prop));
    }

    #[test]
    fn let_binding_substitutes_the_value_into_the_result_type() {
        // let x : Type 1 := Type 0 in Eq(Type 1, x, x) : Prop
        // (Type 0 : Type 1, so the value matches its declared type;
        // the point of the test is that the body's inferred type
        // doesn't leak the local `x` once it's substituted away.)
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let x0 = arena.bound_var(0);
        let eq_body = arena.eq(ty1, x0, x0);
        let let_term = arena.let_(x, ty1, ty0, eq_body);

        let mut fuel = DEFAULT_FUEL;
        let ty = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            let_term,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(ty, arena.sort(Sort::Prop));
    }

    #[test]
    fn equality_of_mismatched_operand_types_is_rejected() {
        // Eq(Type 0, Type 0, Type 1) with lhs/rhs claimed to have type
        // `Type 0`, but `Type 1 : Type 2`, not `Type 0` -- rhs fails
        // to check against the stated equality type.
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let bad_eq = arena.eq(ty0, ty0, ty1);

        let mut fuel = DEFAULT_FUEL;
        let err = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            bad_eq,
            &mut fuel,
        )
        .unwrap_err();
        assert!(matches!(err, TypeError::TypeMismatch { .. }));
    }

    #[test]
    fn applying_a_non_function_is_rejected() {
        // (Type 0) (Type 0) -- Type 0 is not a Pi type.
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let bogus_app = arena.app(ty0, ty0);
        let mut fuel = DEFAULT_FUEL;
        let err = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            bogus_app,
            &mut fuel,
        )
        .unwrap_err();
        assert!(matches!(err, TypeError::NotAFunction { .. }));
    }

    #[test]
    fn an_unbound_free_variable_is_rejected() {
        let (mut arena, mut interner) = setup();
        let sym = interner.intern("nowhere_bound");
        let free = arena.free_var(sym);
        let mut fuel = DEFAULT_FUEL;
        let err = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            free,
            &mut fuel,
        )
        .unwrap_err();
        assert_eq!(err, TypeError::UnboundVariable(sym));
    }

    #[test]
    fn an_undeclared_constant_is_rejected() {
        let (mut arena, mut interner) = setup();
        let sym = interner.intern("mystery");
        let c = arena.const_(sym);
        let mut fuel = DEFAULT_FUEL;
        let err = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            c,
            &mut fuel,
        )
        .unwrap_err();
        assert_eq!(err, TypeError::UnknownConstant(sym));
    }

    #[test]
    fn a_loose_bound_variable_is_rejected_rather_than_indexing_out_of_scope() {
        let (mut arena, mut interner) = setup();
        let stray = arena.bound_var(0);
        let mut fuel = DEFAULT_FUEL;
        let err = infer(
            &mut arena,
            &mut interner,
            &NoGlobals,
            &Context::new(),
            stray,
            &mut fuel,
        )
        .unwrap_err();
        assert_eq!(err, TypeError::UnexpectedLooseBoundVar);
    }
}
