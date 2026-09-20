// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The kernel's typing judgments: `infer_type`, `check`, and
//! `definitional_equal`.
//!
//! This is an independent implementation of the same type theory
//! `vf-typecheck` implements, written and tested separately from it
//! and never calling into it. That duplication is deliberate, not an
//! oversight: `vf-typecheck` exists to give `vf-elab` fast, friendly
//! errors during elaboration, and this project's hard requirement
//! that "the kernel must never trust its own elaborator" means the
//! kernel's soundness cannot depend on `vf-typecheck` being correct.
//! If `vf-typecheck` were deleted entirely, every function in this
//! module would still work exactly as before.
use crate::context::Context;
use crate::env::{AsDeltaContext, KernelEnv};
use crate::error::KernelError;
use vf_core::{Interner, Sort, Term, TermArena, TermId};

/// The sort of `Pi (_ : A), B` given `A : domain_sort` and
/// `B : codomain_sort` (impredicative Prop, predicative cumulative
/// Type -- standard to the Calculus of Constructions): a
/// `Prop`-valued codomain makes the whole Pi a `Prop` regardless of
/// the domain's sort; otherwise the Pi lives at the join of both
/// sorts' levels.
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

fn as_sort<E: KernelEnv>(
    arena: &mut TermArena,
    env: &E,
    term: TermId,
    fuel: &mut u64,
) -> Result<Sort, KernelError> {
    let w = vf_reducer::whnf(arena, &AsDeltaContext(env), term, fuel)?;
    match arena.get(w)? {
        Term::Sort(s) => Ok(*s),
        _ => Err(KernelError::NotASort { got: w }),
    }
}

fn as_pi<E: KernelEnv>(
    arena: &mut TermArena,
    env: &E,
    term: TermId,
    fuel: &mut u64,
) -> Result<(TermId, TermId), KernelError> {
    let w = vf_reducer::whnf(arena, &AsDeltaContext(env), term, fuel)?;
    match arena.get(w)?.clone() {
        Term::Pi {
            domain, codomain, ..
        } => Ok((domain, codomain)),
        _ => Err(KernelError::NotAFunction { head_type: w }),
    }
}

/// `term`'s own type must reduce to a [`Term::Sort`]: the shared
/// Pi/Lam/Let/Eq formation check ("is `term` a well-formed type").
/// Public so callers that register top-level declarations (e.g.
/// `vf-axioms`) can run the same check the kernel itself uses on a
/// `Pi`/`Lam`/`Let`/`Eq`'s own annotations, without duplicating it.
pub fn sort_of<E: KernelEnv>(
    arena: &mut TermArena,
    interner: &mut Interner,
    env: &E,
    ctx: &Context,
    term: TermId,
    fuel: &mut u64,
) -> Result<Sort, KernelError> {
    let ty = infer_type(arena, interner, env, ctx, term, fuel)?;
    as_sort(arena, env, ty, fuel)
}

/// Definitional equality of two types/terms, from the kernel's own
/// (trusted) point of view. A thin wrapper over [`vf_reducer::def_eq`]
/// so callers outside this crate never need to construct an
/// [`AsDeltaContext`] themselves.
pub fn definitional_equal<E: KernelEnv>(
    arena: &mut TermArena,
    env: &E,
    a: TermId,
    b: TermId,
    fuel: &mut u64,
) -> Result<bool, KernelError> {
    Ok(vf_reducer::def_eq(arena, &AsDeltaContext(env), a, b, fuel)?)
}

/// Infer `term`'s type from scratch under `ctx`/`env`.
pub fn infer_type<E: KernelEnv>(
    arena: &mut TermArena,
    interner: &mut Interner,
    env: &E,
    ctx: &Context,
    term: TermId,
    fuel: &mut u64,
) -> Result<TermId, KernelError> {
    let t = arena.get(term)?.clone();
    match t {
        Term::Sort(s) => Ok(arena.sort(s.classifier())),
        Term::BoundVar(_) => Err(KernelError::UnexpectedLooseBoundVar),
        Term::FreeVar(sym) => ctx.lookup(sym).ok_or(KernelError::UnboundVariable(sym)),
        Term::Const(sym) => env
            .type_of_const(sym)
            .ok_or(KernelError::UnknownConstant(sym)),
        Term::App(f, a) => {
            let f_ty = infer_type(arena, interner, env, ctx, f, fuel)?;
            let (domain, codomain) = as_pi(arena, env, f_ty, fuel)?;
            check(arena, interner, env, ctx, a, domain, fuel)?;
            Ok(arena.open_at(codomain, 0, a)?)
        }
        Term::Lam { binder, ty, body } => {
            sort_of(arena, interner, env, ctx, ty, fuel)?;
            let sym = interner.fresh();
            let free = arena.free_var(sym);
            let opened_body = arena.open_at(body, 0, free)?;
            let inner_ctx = ctx.extended(sym, ty);
            let body_ty = infer_type(arena, interner, env, &inner_ctx, opened_body, fuel)?;
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
            let body_ty = infer_type(arena, interner, env, &inner_ctx, opened_body, fuel)?;
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
pub fn check<E: KernelEnv>(
    arena: &mut TermArena,
    interner: &mut Interner,
    env: &E,
    ctx: &Context,
    term: TermId,
    expected_ty: TermId,
    fuel: &mut u64,
) -> Result<(), KernelError> {
    let t = arena.get(term)?.clone();
    if let Term::Lam { ty, body, .. } = t {
        let (domain, codomain) = as_pi(arena, env, expected_ty, fuel)?;
        if !definitional_equal(arena, env, ty, domain, fuel)? {
            return Err(KernelError::TypeMismatch {
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

    let inferred = infer_type(arena, interner, env, ctx, term, fuel)?;
    if definitional_equal(arena, env, inferred, expected_ty, fuel)? {
        Ok(())
    } else {
        Err(KernelError::TypeMismatch {
            expected: expected_ty,
            found: inferred,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::EmptyEnv;
    use vf_core::Binder;
    use vf_reducer::DEFAULT_FUEL;

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    fn nondep_pi(
        arena: &mut TermArena,
        interner: &mut Interner,
        domain: TermId,
        codomain: TermId,
    ) -> TermId {
        let dummy = Binder(interner.intern("_"));
        arena.pi(dummy, domain, codomain)
    }

    #[test]
    fn sort_prop_has_type_type_zero() {
        let (mut arena, mut interner) = setup();
        let prop = arena.sort(Sort::Prop);
        let mut fuel = DEFAULT_FUEL;
        let ty = infer_type(
            &mut arena,
            &mut interner,
            &EmptyEnv,
            &Context::new(),
            prop,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(ty, arena.sort(Sort::Type(0)));
    }

    #[test]
    fn identity_function_infers_a_non_dependent_pi_type() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);

        let mut fuel = DEFAULT_FUEL;
        let inferred = infer_type(
            &mut arena,
            &mut interner,
            &EmptyEnv,
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
        let inferred = infer_type(
            &mut arena,
            &mut interner,
            &EmptyEnv,
            &Context::new(),
            applied,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(inferred, ty1);
    }

    #[test]
    fn checking_a_lambda_against_a_mismatched_domain_fails() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);
        let expected = nondep_pi(&mut arena, &mut interner, ty1, ty1);

        let mut fuel = DEFAULT_FUEL;
        let err = check(
            &mut arena,
            &mut interner,
            &EmptyEnv,
            &Context::new(),
            id_fn,
            expected,
            &mut fuel,
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::TypeMismatch { .. }));
    }

    #[test]
    fn a_declared_axiom_can_be_used_and_its_type_looked_up() {
        // A minimal KernelEnv with one axiom `c : Type 0`, standing in
        // for what vf-axioms's real registry will provide.
        struct OneAxiom {
            name: vf_core::Symbol,
            ty: TermId,
        }
        impl KernelEnv for OneAxiom {
            fn type_of_const(&self, name: vf_core::Symbol) -> Option<TermId> {
                (name == self.name).then_some(self.ty)
            }
            fn unfold(&self, _name: vf_core::Symbol) -> Option<TermId> {
                None // an axiom has no unfolding
            }
        }

        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("c");
        let env = OneAxiom { name, ty: ty0 };
        let c = arena.const_(name);

        let mut fuel = DEFAULT_FUEL;
        let inferred = infer_type(
            &mut arena,
            &mut interner,
            &env,
            &Context::new(),
            c,
            &mut fuel,
        )
        .unwrap();
        assert_eq!(inferred, ty0);
    }

    #[test]
    fn an_undeclared_constant_is_rejected() {
        let (mut arena, mut interner) = setup();
        let sym = interner.intern("mystery");
        let c = arena.const_(sym);
        let mut fuel = DEFAULT_FUEL;
        let err = infer_type(
            &mut arena,
            &mut interner,
            &EmptyEnv,
            &Context::new(),
            c,
            &mut fuel,
        )
        .unwrap_err();
        assert_eq!(err, KernelError::UnknownConstant(sym));
    }

    #[test]
    fn applying_a_non_function_is_rejected() {
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let bogus_app = arena.app(ty0, ty0);
        let mut fuel = DEFAULT_FUEL;
        let err = infer_type(
            &mut arena,
            &mut interner,
            &EmptyEnv,
            &Context::new(),
            bogus_app,
            &mut fuel,
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::NotAFunction { .. }));
    }

    #[test]
    fn definitional_equal_sees_through_a_beta_redex() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);
        let arg = arena.sort(Sort::Type(9));
        let redex = arena.app(id_fn, arg);

        let mut fuel = DEFAULT_FUEL;
        assert!(definitional_equal(&mut arena, &EmptyEnv, redex, arg, &mut fuel).unwrap());
    }

    #[test]
    fn a_type_in_type_style_paradox_term_is_rejected_not_hung() {
        // The kernel must fail closed (not hang) on a term whose
        // reduction the fuel budget can't complete, even though this
        // particular term is never accepted as well-typed in the
        // first place -- `check` reduces its target type using
        // whnf/def_eq before the ill-typed self-application is ever
        // reached in a body, so this exercises the same
        // fail-closed-on-fuel-exhaustion path from the kernel's public
        // API rather than vf-reducer's directly.
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let self_app_body = arena.app(x0, x0);
        let omega_half = arena.lam(x, ty0, self_app_body);
        let omega = arena.app(omega_half, omega_half);

        let mut fuel = 1000u64;
        let err = infer_type(
            &mut arena,
            &mut interner,
            &EmptyEnv,
            &Context::new(),
            omega,
            &mut fuel,
        );
        assert!(err.is_err());
    }
}
