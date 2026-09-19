//! Symbolic calculus judgments: shared term-building infrastructure for
//! the *statement shapes* a symbolic calculus theory typically wants
//! -- evaluation, equivalence, and differentiation -- parallel to
//! `vf-algebra`'s law-statement builders but for expression-oriented
//! theories instead of pure algebraic structures.
//!
//! ## What this crate is (and isn't)
//!
//! Exactly like `vf-algebra`: this crate only builds `vf-core::Term`s
//! (`vf-core::Term::Eq` propositions), never proves anything and never
//! decides how an expression type, an evaluator, or a derivative
//! function is declared. `vf-workerman` (a specific, user-defined
//! calculus built on top of this crate) supplies all of that and does
//! the actual proving, typically via `vf-axioms::Registry::declare_theorem`.
//!
//! There is no dedicated "typing judgment" builder here: `e : T` is
//! not itself an `Eq`-shaped proposition to build a statement out of,
//! it's the ordinary kernel membership judgment `vf-kernel::check`
//! already answers directly -- adding a wrapper around that would
//! only relocate, not simplify, the call. What's here is the part that
//! genuinely repeats across calculus-flavored theories: stating that
//! something evaluates to something, that two expressions are the
//! same, or that one expression is another's derivative.
#![forbid(unsafe_code)]

use vf_core::TermArena;
pub use vf_core::TermId;

/// `f a b`, avoiding the borrow conflict of nested `arena.app(arena.app(..))`.
fn app2(arena: &mut TermArena, f: TermId, a: TermId, b: TermId) -> TermId {
    let fa = arena.app(f, a);
    arena.app(fa, b)
}

/// "`e` evaluates to `v`": `Eq(value_ty, eval_fn(e), v)`, for a
/// single-argument evaluator (`eval_fn : Expr -> Value`).
pub fn evaluates_to(
    arena: &mut TermArena,
    value_ty: TermId,
    eval_fn: TermId,
    e: TermId,
    v: TermId,
) -> TermId {
    let ev = arena.app(eval_fn, e);
    arena.eq(value_ty, ev, v)
}

/// "`e` evaluates to `v` under input `x`":
/// `Eq(value_ty, eval_fn(e, x), v)`, for a two-argument evaluator
/// (`eval_fn : Expr -> Input -> Value`) -- the common shape when
/// evaluation is parameterized by e.g. a variable's value.
pub fn evaluates_to_under(
    arena: &mut TermArena,
    value_ty: TermId,
    eval_fn: TermId,
    e: TermId,
    x: TermId,
    v: TermId,
) -> TermId {
    let ev = app2(arena, eval_fn, e, x);
    arena.eq(value_ty, ev, v)
}

/// "`e1` and `e2` are the same expression": `Eq(expr_ty, e1, e2)`.
/// Deliberately just `Eq` under a calculus-flavored name -- a
/// symbolic calculus's notion of "equivalent" is ordinary term
/// equality on its expression type, not a separate relation needing
/// its own semantics.
pub fn equivalent(arena: &mut TermArena, expr_ty: TermId, e1: TermId, e2: TermId) -> TermId {
    arena.eq(expr_ty, e1, e2)
}

/// "`e_prime` is the derivative of `e`": `Eq(expr_ty, deriv_fn(e), e_prime)`.
pub fn derivative_of(
    arena: &mut TermArena,
    expr_ty: TermId,
    deriv_fn: TermId,
    e: TermId,
    e_prime: TermId,
) -> TermId {
    let de = arena.app(deriv_fn, e);
    arena.eq(expr_ty, de, e_prime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_core::{Interner, Sort, Term};

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    fn const_(arena: &mut TermArena, interner: &mut Interner, name: &str) -> TermId {
        arena.const_(interner.intern(name))
    }

    #[test]
    fn evaluates_to_builds_an_eq_of_the_applied_evaluator() {
        let (mut arena, mut interner) = setup();
        let value_ty = arena.sort(Sort::Type(0));
        let eval_fn = const_(&mut arena, &mut interner, "eval");
        let e = const_(&mut arena, &mut interner, "e");
        let v = const_(&mut arena, &mut interner, "v");
        let stmt = evaluates_to(&mut arena, value_ty, eval_fn, e, v);
        let Term::Eq { ty, lhs, rhs } = arena.get(stmt).unwrap().clone() else {
            panic!("expected an Eq");
        };
        assert_eq!(ty, value_ty);
        assert_eq!(rhs, v);
        assert_eq!(arena.get(lhs).unwrap(), &Term::App(eval_fn, e));
    }

    #[test]
    fn evaluates_to_under_curries_both_arguments() {
        let (mut arena, mut interner) = setup();
        let value_ty = arena.sort(Sort::Type(0));
        let eval_fn = const_(&mut arena, &mut interner, "eval");
        let e = const_(&mut arena, &mut interner, "e");
        let x = const_(&mut arena, &mut interner, "x");
        let v = const_(&mut arena, &mut interner, "v");
        let stmt = evaluates_to_under(&mut arena, value_ty, eval_fn, e, x, v);
        let Term::Eq { lhs, .. } = arena.get(stmt).unwrap().clone() else {
            panic!("expected an Eq");
        };
        let Term::App(inner, arg2) = arena.get(lhs).unwrap().clone() else {
            panic!("expected an outer App");
        };
        assert_eq!(arg2, x);
        assert_eq!(arena.get(inner).unwrap(), &Term::App(eval_fn, e));
    }

    #[test]
    fn equivalent_is_plain_eq_on_the_expression_type() {
        let (mut arena, mut interner) = setup();
        let expr_ty = const_(&mut arena, &mut interner, "Expr");
        let e1 = const_(&mut arena, &mut interner, "e1");
        let e2 = const_(&mut arena, &mut interner, "e2");
        let stmt = equivalent(&mut arena, expr_ty, e1, e2);
        assert_eq!(
            arena.get(stmt).unwrap(),
            &Term::Eq {
                ty: expr_ty,
                lhs: e1,
                rhs: e2
            }
        );
    }

    #[test]
    fn derivative_of_builds_an_eq_of_the_applied_deriv_function() {
        let (mut arena, mut interner) = setup();
        let expr_ty = const_(&mut arena, &mut interner, "Expr");
        let deriv_fn = const_(&mut arena, &mut interner, "deriv");
        let e = const_(&mut arena, &mut interner, "e");
        let e_prime = const_(&mut arena, &mut interner, "e_prime");
        let stmt = derivative_of(&mut arena, expr_ty, deriv_fn, e, e_prime);
        let Term::Eq { ty, lhs, rhs } = arena.get(stmt).unwrap().clone() else {
            panic!("expected an Eq");
        };
        assert_eq!(ty, expr_ty);
        assert_eq!(rhs, e_prime);
        assert_eq!(arena.get(lhs).unwrap(), &Term::App(deriv_fn, e));
    }
}
