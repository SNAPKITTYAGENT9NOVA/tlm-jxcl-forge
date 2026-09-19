//! Beta/zeta/delta reduction and definitional equality over
//! `vf-core::Term`, with a fuel budget instead of a termination proof.
//!
//! ## Trust status
//!
//! This crate is part of the *trusted* computing base described in
//! `docs/TRUST_MODEL.md`: `vf-kernel` calls [`def_eq`] to decide
//! whether two types match, and has no independent way to check that
//! decision. Its algorithm is therefore kept as simple as possible
//! (normalize both sides fully, then compare) rather than the faster
//! lazy structural comparison a production kernel (e.g. Lean's) would
//! use -- a slower, more obviously correct algorithm is the right
//! trade-off for something callers cannot re-verify.
//!
//! ## Known, documented simplifications (not bugs)
//!
//! - **Iota reduction is generic, table-driven primitive recursion.**
//!   [`whnf`]/[`normalize`] perform beta, zeta, delta, *and* iota
//!   reduction, but this crate has no idea what `Nat` or `List` are --
//!   it only knows the shape "a fully-applied recursor whose major
//!   premise is constructor-headed reduces to that constructor's case
//!   function, applied to the constructor's arguments plus a
//!   recursive sub-call for each recursive argument", described
//!   generically via [`RecursorSpec`]/[`ConstructorSpec`]. `vf-axioms`
//!   supplies the specs for the actual built-in types. See
//!   [`iota`]'s module doc for the full mechanism, and its doc comment
//!   on why this is enough to support indexed families (`Vector`,
//!   `Fin`) too, not just simple ones.
//! - **No eta reduction.** `fun (x : T) => f x` is not identified with
//!   `f` even when `x` does not occur free in `f`. This is a real
//!   restriction on what [`def_eq`] can prove equal, not an oversight.
//! - **Fuel, not a normalization proof.** A real dependent type theory
//!   is strongly normalizing for well-typed terms, so a production
//!   kernel can normalize without a step bound. This kernel does not
//!   attempt that metatheory: every reduction function takes a `fuel`
//!   budget and returns [`ReduceError::FuelExhausted`] rather than
//!   looping forever on a term that (whether through a soundness bug
//!   elsewhere or a malicious/malformed input) fails to normalize.
//!   This is a deliberate fail-closed choice, consistent with this
//!   project's hard requirement to never hang instead of reporting an
//!   error.
#![forbid(unsafe_code)]

mod iota;

use std::fmt;
use vf_core::{ArenaError, Symbol, Term, TermArena, TermId};

pub use iota::{ConstructorSpec, RecursorSpec};

/// The default step budget passed to [`whnf`]/[`normalize`]/[`def_eq`]
/// by callers that don't have a more specific bound in mind.
pub const DEFAULT_FUEL: u64 = 1_000_000;

/// How to unfold a [`vf_core::Term::Const`] during delta reduction.
/// `vf-reducer` itself knows nothing about what global declarations
/// exist -- `vf-axioms`/`vf-kernel` own that -- so every reduction
/// entry point is generic over this trait instead of taking a concrete
/// environment type.
pub trait DeltaContext {
    /// The definition `name` unfolds to, or `None` if `name` has no
    /// definition to unfold to (an axiom, an opaque declaration, or an
    /// unknown name -- all three are indistinguishable to a reducer
    /// that only ever *unfolds*, never *checks*, a name).
    fn unfold(&self, name: Symbol) -> Option<TermId>;

    /// If `name` is a registered recursor, its reduction shape.
    /// Defaulted to `None` so implementors with no inductive types to
    /// register (the common case: [`NoDelta`], most tests) need no
    /// changes at all; `vf-axioms`'s registry is the one real override.
    fn recursor(&self, _name: Symbol) -> Option<&RecursorSpec> {
        None
    }

    /// If `name` is a registered constructor, which recursor (by its
    /// `Const` symbol) it belongs to and its 0-based index among that
    /// recursor's constructors. Also defaulted to `None`.
    fn constructor_index(&self, _name: Symbol) -> Option<(Symbol, usize)> {
        None
    }
}

/// A [`DeltaContext`] with no unfoldable definitions at all. Useful in
/// tests, and for reducing terms that are known to be axiom-only.
pub struct NoDelta;

impl DeltaContext for NoDelta {
    fn unfold(&self, _name: Symbol) -> Option<TermId> {
        None
    }
}

/// Everything that can go wrong while reducing a term. Never a panic:
/// an out-of-range [`TermId`] or an exhausted fuel budget is reported,
/// not `unwrap()`-ed past.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReduceError {
    Arena(ArenaError),
    /// The reduction did not reach a normal form (or whnf) within the
    /// supplied fuel budget. See this module's doc comment.
    FuelExhausted,
}

impl From<ArenaError> for ReduceError {
    fn from(e: ArenaError) -> Self {
        ReduceError::Arena(e)
    }
}

impl fmt::Display for ReduceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReduceError::Arena(e) => write!(f, "{e}"),
            ReduceError::FuelExhausted => {
                write!(
                    f,
                    "reduction exceeded its fuel budget without reaching a normal form"
                )
            }
        }
    }
}
impl std::error::Error for ReduceError {}

fn consume(fuel: &mut u64) -> Result<(), ReduceError> {
    if *fuel == 0 {
        return Err(ReduceError::FuelExhausted);
    }
    *fuel -= 1;
    Ok(())
}

/// Reduce `term` to weak head normal form: keep rewriting the
/// outermost redex (beta/zeta/delta/iota) until the head is a
/// variable, a constant with no unfolding, a sort, a lambda, a pi, or
/// an equality -- i.e. until no more progress can be made *at the
/// top* without looking inside a binder.
pub fn whnf<C: DeltaContext>(
    arena: &mut TermArena,
    ctx: &C,
    term: TermId,
    fuel: &mut u64,
) -> Result<TermId, ReduceError> {
    let mut current = term;
    loop {
        if let Some(reduced) = iota::try_iota_reduce(arena, ctx, current, fuel)? {
            current = reduced;
            continue;
        }
        let t = arena.get(current)?.clone();
        match t {
            Term::App(f, a) => {
                let f_whnf = whnf(arena, ctx, f, fuel)?;
                if let Term::Lam { body, .. } = arena.get(f_whnf)?.clone() {
                    consume(fuel)?;
                    current = arena.open_at(body, 0, a)?;
                    continue;
                }
                // `f_whnf` is in whnf and not a lambda, so `App(f_whnf,
                // a)` is stuck: this is the result, whether or not `f`
                // itself needed rewriting to reach `f_whnf`.
                return Ok(if f_whnf == f {
                    current
                } else {
                    arena.app(f_whnf, a)
                });
            }
            Term::Let { value, body, .. } => {
                consume(fuel)?;
                current = arena.open_at(body, 0, value)?;
                continue;
            }
            Term::Const(name) => match ctx.unfold(name) {
                Some(def) => {
                    consume(fuel)?;
                    current = def;
                    continue;
                }
                None => return Ok(current),
            },
            _ => return Ok(current),
        }
    }
}

/// Reduce `term` to full normal form: [`whnf`] at every position,
/// including inside binder bodies/domains/codomains.
pub fn normalize<C: DeltaContext>(
    arena: &mut TermArena,
    ctx: &C,
    term: TermId,
    fuel: &mut u64,
) -> Result<TermId, ReduceError> {
    let head = whnf(arena, ctx, term, fuel)?;
    let t = arena.get(head)?.clone();
    Ok(match t {
        Term::Sort(_) | Term::BoundVar(_) | Term::FreeVar(_) | Term::Const(_) => head,
        Term::App(f, a) => {
            let f2 = normalize(arena, ctx, f, fuel)?;
            let a2 = normalize(arena, ctx, a, fuel)?;
            arena.app(f2, a2)
        }
        Term::Lam { binder, ty, body } => {
            let ty2 = normalize(arena, ctx, ty, fuel)?;
            let body2 = normalize(arena, ctx, body, fuel)?;
            arena.lam(binder, ty2, body2)
        }
        Term::Pi {
            binder,
            domain,
            codomain,
        } => {
            let d2 = normalize(arena, ctx, domain, fuel)?;
            let c2 = normalize(arena, ctx, codomain, fuel)?;
            arena.pi(binder, d2, c2)
        }
        // whnf always zeta-reduces a head Let, so `head` is never
        // itself a Let -- kept exhaustive (rather than `unreachable!`)
        // so a future change to `whnf` can't silently make this a
        // logic bug instead of a compile error.
        Term::Let {
            binder,
            ty,
            value,
            body,
        } => {
            let ty2 = normalize(arena, ctx, ty, fuel)?;
            let v2 = normalize(arena, ctx, value, fuel)?;
            let b2 = normalize(arena, ctx, body, fuel)?;
            arena.let_(binder, ty2, v2, b2)
        }
        Term::Eq { ty, lhs, rhs } => {
            let ty2 = normalize(arena, ctx, ty, fuel)?;
            let l2 = normalize(arena, ctx, lhs, fuel)?;
            let r2 = normalize(arena, ctx, rhs, fuel)?;
            arena.eq(ty2, l2, r2)
        }
    })
}

/// Definitional equality: are `a` and `b` equal up to beta/zeta/delta
/// reduction? See this module's doc comment for why this is
/// "normalize both sides, then compare" rather than a faster lazy
/// algorithm.
pub fn def_eq<C: DeltaContext>(
    arena: &mut TermArena,
    ctx: &C,
    a: TermId,
    b: TermId,
    fuel: &mut u64,
) -> Result<bool, ReduceError> {
    if a == b {
        // Hash-consing shortcut: syntactically identical terms are
        // trivially definitionally equal without spending any fuel.
        return Ok(true);
    }
    let na = normalize(arena, ctx, a, fuel)?;
    let nb = normalize(arena, ctx, b, fuel)?;
    Ok(na == nb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use vf_core::{Binder, Interner, Sort};

    struct MapDelta(HashMap<Symbol, TermId>);
    impl DeltaContext for MapDelta {
        fn unfold(&self, name: Symbol) -> Option<TermId> {
            self.0.get(&name).copied()
        }
    }

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    #[test]
    fn whnf_on_a_normal_form_is_a_no_op() {
        let (mut arena, _) = setup();
        let s = arena.sort(Sort::Type(0));
        let mut fuel = DEFAULT_FUEL;
        assert_eq!(whnf(&mut arena, &NoDelta, s, &mut fuel).unwrap(), s);
    }

    #[test]
    fn beta_reduces_an_applied_identity_lambda() {
        // (fun (x : Type 0) => x) (Type 5)  ~>  Type 5
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let body = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, body);
        let arg = arena.sort(Sort::Type(5));
        let app = arena.app(id_fn, arg);

        let mut fuel = DEFAULT_FUEL;
        let result = whnf(&mut arena, &NoDelta, app, &mut fuel).unwrap();
        assert_eq!(result, arg);
    }

    #[test]
    fn zeta_reduces_a_let_binding() {
        // let x : Type 0 := Type 5 in x  ~>  Type 5
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let value = arena.sort(Sort::Type(5));
        let body = arena.bound_var(0);
        let let_term = arena.let_(x, ty0, value, body);

        let mut fuel = DEFAULT_FUEL;
        let result = whnf(&mut arena, &NoDelta, let_term, &mut fuel).unwrap();
        assert_eq!(result, value);
    }

    #[test]
    fn delta_unfolds_a_defined_constant() {
        let (mut arena, mut interner) = setup();
        let name = interner.intern("myconst");
        let definition = arena.sort(Sort::Type(7));
        let const_term = arena.const_(name);
        let mut defs = HashMap::new();
        defs.insert(name, definition);
        let ctx = MapDelta(defs);

        let mut fuel = DEFAULT_FUEL;
        let result = whnf(&mut arena, &ctx, const_term, &mut fuel).unwrap();
        assert_eq!(result, definition);
    }

    #[test]
    fn an_axiom_like_constant_with_no_definition_is_already_whnf() {
        let (mut arena, mut interner) = setup();
        let name = interner.intern("my_axiom");
        let const_term = arena.const_(name);
        let mut fuel = DEFAULT_FUEL;
        assert_eq!(
            whnf(&mut arena, &NoDelta, const_term, &mut fuel).unwrap(),
            const_term
        );
    }

    #[test]
    fn normalize_reduces_inside_a_lambda_body() {
        // fun (_ : Type 0) => (fun (y : Type 0) => y) (Type 3)
        //   normalizes to fun (_ : Type 0) => Type 3
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let y = Binder(interner.intern("y"));
        let ty0 = arena.sort(Sort::Type(0));
        let ty3 = arena.sort(Sort::Type(3));

        let y0 = arena.bound_var(0);
        let inner_id = arena.lam(y, ty0, y0);
        let inner_app = arena.app(inner_id, ty3);
        let outer = arena.lam(x, ty0, inner_app);

        let mut fuel = DEFAULT_FUEL;
        let normalized = normalize(&mut arena, &NoDelta, outer, &mut fuel).unwrap();
        let expected = arena.lam(x, ty0, ty3);
        assert_eq!(normalized, expected);
    }

    #[test]
    fn def_eq_identifies_a_beta_redex_with_its_reduct() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);
        let arg = arena.sort(Sort::Type(9));
        let redex = arena.app(id_fn, arg);

        let mut fuel = DEFAULT_FUEL;
        assert!(def_eq(&mut arena, &NoDelta, redex, arg, &mut fuel).unwrap());
    }

    #[test]
    fn def_eq_rejects_two_genuinely_different_terms() {
        let (mut arena, _) = setup();
        let a = arena.sort(Sort::Type(0));
        let b = arena.sort(Sort::Type(1));
        let mut fuel = DEFAULT_FUEL;
        assert!(!def_eq(&mut arena, &NoDelta, a, b, &mut fuel).unwrap());
    }

    #[test]
    fn zero_fuel_fails_closed_instead_of_looping() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let id_fn = arena.lam(x, ty0, x0);
        let arg = arena.sort(Sort::Type(9));
        let redex = arena.app(id_fn, arg);

        let mut fuel = 0u64;
        assert_eq!(
            whnf(&mut arena, &NoDelta, redex, &mut fuel),
            Err(ReduceError::FuelExhausted)
        );
    }

    #[test]
    fn a_non_terminating_self_application_fails_closed_rather_than_hanging() {
        // (fun (x : Type 0) => x x) (fun (x : Type 0) => x x) never
        // reaches a normal form; a fuel-bounded reducer must report
        // FuelExhausted rather than loop forever. This term is
        // ill-typed (vf-kernel would reject it before ever normalizing
        // it), but vf-reducer itself must still fail closed on it.
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let x0 = arena.bound_var(0);
        let self_app_body = arena.app(x0, x0);
        let omega_half = arena.lam(x, ty0, self_app_body);
        let omega = arena.app(omega_half, omega_half);

        let mut fuel = 1000u64;
        assert_eq!(
            whnf(&mut arena, &NoDelta, omega, &mut fuel),
            Err(ReduceError::FuelExhausted)
        );
    }
}
