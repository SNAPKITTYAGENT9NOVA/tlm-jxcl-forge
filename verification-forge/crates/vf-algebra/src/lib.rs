//! Generic algebraic-structure/law framework: term-building helpers
//! for the *statement* of common algebraic laws (associativity,
//! commutativity, identities, involutions, ...), parameterized over a
//! carrier type and operation(s) supplied by the caller.
//!
//! ## What this crate is (and isn't)
//!
//! This crate builds `vf-core::Term`s -- `Pi`-quantified
//! `vf-core::Term::Eq` propositions -- and nothing else. It has no
//! opinion about how a carrier or operation is declared (an axiom, a
//! definition, a builtin -- see `vf-axioms`), and it never constructs
//! a *proof* of any law it states: proving a law true for a
//! particular carrier/operation is the caller's job (typically via
//! `vf_axioms::Registry::declare_theorem`), using whatever proof
//! technique applies (often `vf-reducer`'s beta/iota computation
//! directly, or a case-split via a recursor). A "law" in this crate is
//! purely a *statement shape* -- a reusable building block for any
//! algebraic theory, including but not limited to `vf-elucidian`'s.
//!
//! Every function here assumes (and never checks) that `op`/`f`
//! already has the type its usage implies (e.g. `carrier -> carrier
//! -> carrier` for a binary operation) -- callers get that checked for
//! free the moment they hand the resulting statement to
//! `vf-kernel`/`vf-axioms` as a theorem's stated type, since an
//! ill-typed application inside the statement itself would make the
//! statement ill-formed and rejected before any proof is even
//! considered.
#![forbid(unsafe_code)]

use vf_core::{ArenaError, Binder, Interner, TermArena, TermId};

/// `Pi (hint : domain), codomain`, where `codomain` is built from a
/// fresh placeholder standing in for the bound variable. See
/// `vf-core::term::Binder`'s doc comment for why reusing a readable
/// `hint` across calls is safe (binder names never affect term
/// identity).
fn dep_pi(
    arena: &mut TermArena,
    interner: &mut Interner,
    hint: &str,
    domain: TermId,
    build_codomain: impl FnOnce(&mut TermArena, &mut Interner, TermId) -> Result<TermId, ArenaError>,
) -> Result<TermId, ArenaError> {
    let sym = interner.intern(hint);
    let placeholder = arena.free_var(sym);
    let codomain_open = build_codomain(arena, interner, placeholder)?;
    let codomain_closed = arena.close_at(codomain_open, 0, sym)?;
    Ok(arena.pi(Binder(sym), domain, codomain_closed))
}

/// `f a b`, avoiding the borrow conflict of nested `arena.app(arena.app(..))`.
fn app2(arena: &mut TermArena, f: TermId, a: TermId, b: TermId) -> TermId {
    let fa = arena.app(f, a);
    arena.app(fa, b)
}

/// `Pi (x y z : carrier), Eq(carrier, op(op(x,y),z), op(x,op(y,z)))`.
pub fn associative(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    op: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, interner, x| {
        dep_pi(arena, interner, "y", carrier, move |arena, interner, y| {
            dep_pi(arena, interner, "z", carrier, move |arena, _interner, z| {
                let xy = app2(arena, op, x, y);
                let lhs = app2(arena, op, xy, z);
                let yz = app2(arena, op, y, z);
                let rhs = app2(arena, op, x, yz);
                Ok(arena.eq(carrier, lhs, rhs))
            })
        })
    })
}

/// `Pi (x y : carrier), Eq(carrier, op(x,y), op(y,x))`.
pub fn commutative(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    op: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, interner, x| {
        dep_pi(arena, interner, "y", carrier, move |arena, _interner, y| {
            let lhs = app2(arena, op, x, y);
            let rhs = app2(arena, op, y, x);
            Ok(arena.eq(carrier, lhs, rhs))
        })
    })
}

/// `Pi (x : carrier), Eq(carrier, op(e,x), x)`.
pub fn left_identity(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    op: TermId,
    e: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, _interner, x| {
        let lhs = app2(arena, op, e, x);
        Ok(arena.eq(carrier, lhs, x))
    })
}

/// `Pi (x : carrier), Eq(carrier, op(x,e), x)`.
pub fn right_identity(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    op: TermId,
    e: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, _interner, x| {
        let lhs = app2(arena, op, x, e);
        Ok(arena.eq(carrier, lhs, x))
    })
}

/// `Pi (x : carrier), Eq(carrier, f(f(x)), x)` -- `f` is an involution.
pub fn involutive(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    f: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, _interner, x| {
        let fx = arena.app(f, x);
        let ffx = arena.app(f, fx);
        Ok(arena.eq(carrier, ffx, x))
    })
}

/// `Pi (x : carrier), Eq(carrier, op(x,x), x)` -- `op` is idempotent.
pub fn idempotent_binary(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    op: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, _interner, x| {
        let xx = arena.app(op, x);
        let xx = arena.app(xx, x);
        Ok(arena.eq(carrier, xx, x))
    })
}

/// `Pi (x y z : carrier), Eq(carrier, op(op(x,y),z), op(x,z))` --
/// `op`'s left argument absorbs whatever its right argument was after
/// a second application (an unusual property most standard binary
/// operations do *not* have; a "always return the last thing combined
/// with the original left operand" law).
pub fn left_absorbing(
    arena: &mut TermArena,
    interner: &mut Interner,
    carrier: TermId,
    op: TermId,
) -> Result<TermId, ArenaError> {
    dep_pi(arena, interner, "x", carrier, move |arena, interner, x| {
        dep_pi(arena, interner, "y", carrier, move |arena, interner, y| {
            dep_pi(arena, interner, "z", carrier, move |arena, _interner, z| {
                let xy = app2(arena, op, x, y);
                let lhs = app2(arena, op, xy, z);
                let rhs = app2(arena, op, x, z);
                Ok(arena.eq(carrier, lhs, rhs))
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_core::Sort;

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    fn binary_op(arena: &mut TermArena, interner: &mut Interner, name: &str) -> TermId {
        arena.const_(interner.intern(name))
    }

    #[test]
    fn associative_builds_a_pi_pi_pi_eq_shape() {
        let (mut arena, mut interner) = setup();
        let carrier = arena.sort(Sort::Type(0));
        let op = binary_op(&mut arena, &mut interner, "op");
        let stmt = associative(&mut arena, &mut interner, carrier, op).unwrap();
        // Three nested Pi binders over `carrier` before reaching the Eq.
        let mut t = stmt;
        for _ in 0..3 {
            match arena.get(t).unwrap().clone() {
                vf_core::Term::Pi {
                    domain, codomain, ..
                } => {
                    assert_eq!(domain, carrier);
                    t = codomain;
                }
                other => panic!("expected a Pi, got {other:?}"),
            }
        }
        assert!(matches!(arena.get(t).unwrap(), vf_core::Term::Eq { .. }));
    }

    #[test]
    fn different_carriers_produce_different_statements() {
        let (mut arena, mut interner) = setup();
        let carrier_a = arena.sort(Sort::Type(0));
        let carrier_b = arena.sort(Sort::Type(1));
        let op = binary_op(&mut arena, &mut interner, "op");
        let stmt_a = commutative(&mut arena, &mut interner, carrier_a, op).unwrap();
        let stmt_b = commutative(&mut arena, &mut interner, carrier_b, op).unwrap();
        assert_ne!(stmt_a, stmt_b);
    }

    #[test]
    fn left_and_right_identity_are_distinct_statements() {
        let (mut arena, mut interner) = setup();
        let carrier = arena.sort(Sort::Type(0));
        let op = binary_op(&mut arena, &mut interner, "op");
        let e = binary_op(&mut arena, &mut interner, "e");
        let left = left_identity(&mut arena, &mut interner, carrier, op, e).unwrap();
        let right = right_identity(&mut arena, &mut interner, carrier, op, e).unwrap();
        assert_ne!(left, right);
    }

    #[test]
    fn involutive_and_idempotent_are_well_formed_and_distinct() {
        let (mut arena, mut interner) = setup();
        let carrier = arena.sort(Sort::Type(0));
        let f = binary_op(&mut arena, &mut interner, "f");
        let inv = involutive(&mut arena, &mut interner, carrier, f).unwrap();
        let idem = idempotent_binary(&mut arena, &mut interner, carrier, f).unwrap();
        assert_ne!(inv, idem);
        assert!(matches!(arena.get(inv).unwrap(), vf_core::Term::Pi { .. }));
        assert!(matches!(arena.get(idem).unwrap(), vf_core::Term::Pi { .. }));
    }

    #[test]
    fn calling_the_same_builder_twice_hash_conses_to_the_same_statement() {
        let (mut arena, mut interner) = setup();
        let carrier = arena.sort(Sort::Type(0));
        let op = binary_op(&mut arena, &mut interner, "op");
        let a = left_absorbing(&mut arena, &mut interner, carrier, op).unwrap();
        let b = left_absorbing(&mut arena, &mut interner, carrier, op).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_bogus_symbol_never_leaks_into_the_built_statement() {
        // Sanity that these builders don't accidentally thread the
        // wrong TermId anywhere: swap in a distinguishable Sort as the
        // "operation" and confirm the Eq's own two sides both mention
        // it via App, not some other stray term.
        let (mut arena, mut interner) = setup();
        let carrier = arena.sort(Sort::Type(0));
        let marker_sym = interner.intern("marker_op");
        let op = arena.const_(marker_sym);
        let stmt = commutative(&mut arena, &mut interner, carrier, op).unwrap();
        // Peel both Pis, then check the Eq's lhs/rhs are each an App
        // whose function position is (after peeling one App layer) op.
        let inner = match arena.get(stmt).unwrap().clone() {
            vf_core::Term::Pi { codomain, .. } => match arena.get(codomain).unwrap().clone() {
                vf_core::Term::Pi { codomain, .. } => codomain,
                other => panic!("expected inner Pi, got {other:?}"),
            },
            other => panic!("expected outer Pi, got {other:?}"),
        };
        let (lhs, rhs) = match arena.get(inner).unwrap().clone() {
            vf_core::Term::Eq { lhs, rhs, .. } => (lhs, rhs),
            other => panic!("expected Eq, got {other:?}"),
        };
        for side in [lhs, rhs] {
            // `op(a, b)` is curried: App(App(op, a), b). Peel both
            // layers to reach the head.
            let inner_app = match arena.get(side).unwrap().clone() {
                vf_core::Term::App(f, _) => f,
                other => panic!("expected outer App, got {other:?}"),
            };
            match arena.get(inner_app).unwrap().clone() {
                vf_core::Term::App(f, _) => match arena.get(f).unwrap().clone() {
                    vf_core::Term::Const(s) => assert_eq!(s, marker_sym),
                    other => panic!("expected Const(op), got {other:?}"),
                },
                other => panic!("expected inner App, got {other:?}"),
            }
        }
    }
}
