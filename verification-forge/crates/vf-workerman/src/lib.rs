//! Workerman's Calculus.
//!
//! ## This is not an established mathematical discipline
//!
//! "Workerman's Calculus" is a name invented for this project, exactly
//! like `vf-elucidian`'s "Elucidian Algebra". It is not a citation of,
//! or a claim to formalize, differential calculus, computer algebra,
//! or any other recognized field -- "the Workerman derivative"
//! (`wderiv`) is a small, structurally-recursive rewrite over a
//! four-constructor toy expression type, picked because its rules
//! happen to echo the sum and product rules familiar from calculus,
//! not because this crate is claiming to formalize calculus itself.
//! Nothing here should be read as a contribution to, or a claim about,
//! mathematics as practiced outside this codebase.
//!
//! ## The theory
//!
//! `WExpr`, a small expression type over a single implicit variable
//! `x`:
//! - `wvar : WExpr` -- the variable itself.
//! - `wconst : Nat -> WExpr` -- a constant.
//! - `wadd`, `wmul : WExpr -> WExpr -> WExpr`.
//!
//! with two functions defined on it by structural recursion
//! (`WExpr_rec`):
//! - `weval : WExpr -> Nat -> Nat`, interpreting an expression given a
//!   value for `x`.
//! - `wderiv : WExpr -> WExpr`, Workerman's derivative: `wderiv(wvar)
//!   = wconst 1`, `wderiv(wconst n) = wconst 0`, `wderiv` distributes
//!   over `wadd` (the sum rule), and follows the product rule over
//!   `wmul`.
//!
//! and two theorems, both genuine `vf-kernel`-checked proofs (never
//! axioms):
//! - `wderiv_const_is_zero`: differentiating any constant gives zero,
//!   for every `n : Nat` -- provable by computation alone (see
//!   `wexpr::build_wexpr`'s doc comment on why `n`'s own concreteness
//!   doesn't matter here).
//! - `eval_add_zero_right`: evaluating `e + 0` gives the same result
//!   as evaluating `e`, for *every* `e : WExpr` (not just concrete
//!   ones) -- this one genuinely needs induction, not on `WExpr`
//!   itself, but on the underlying `Nat` arithmetic: it reduces to
//!   `vf-workerman::arith`'s `add_zero_right` lemma, which is proved
//!   by real induction via `Nat_ind` (the classic "n + 0 = n"
//!   exercise). This crate's proof is then just *reusing* that lemma,
//!   not re-deriving it -- exactly how a real development would layer
//!   theorems.
#![forbid(unsafe_code)]

mod arith;
mod helpers;
mod wexpr;

pub use arith::Arith;
pub use wexpr::WExpr;

use vf_axioms::{Prelude, Registry, RegistryError};
use vf_core::{Interner, TermArena};

/// Everything `vf-workerman` registers: `Nat` arithmetic plus `WExpr`,
/// `weval`, `wderiv`, and both theorems.
#[derive(Debug, Clone, Copy)]
pub struct WorkermanCalculus {
    pub arith: Arith,
    pub wexpr: WExpr,
}

/// Register the full Workerman's Calculus theory into `registry`.
/// `prelude` must already be registered (see
/// `vf_axioms::register_prelude`) -- `Nat`'s constructors/recursors,
/// `refl`, and `ap` are all used here.
pub fn build_workerman_calculus(
    registry: &mut Registry,
    prelude: &Prelude,
    arena: &mut TermArena,
    interner: &mut Interner,
    fuel: &mut u64,
) -> Result<WorkermanCalculus, RegistryError> {
    let arith = arith::build_arith(registry, prelude, arena, interner, fuel)?;
    let wexpr = wexpr::build_wexpr(registry, prelude, &arith, arena, interner, fuel)?;
    Ok(WorkermanCalculus { arith, wexpr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_axioms::AxiomPolicy;
    use vf_reducer::DEFAULT_FUEL;

    fn setup() -> (TermArena, Interner, Registry, u64) {
        (
            TermArena::new(),
            Interner::new(),
            Registry::new(AxiomPolicy::NoAxioms),
            DEFAULT_FUEL,
        )
    }

    #[test]
    fn the_whole_theory_registers_and_checks_against_the_kernel() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let theory = build_workerman_calculus(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        for name in [
            theory.arith.add,
            theory.arith.mul,
            theory.arith.add_zero_right,
            theory.wexpr.wexpr,
            theory.wexpr.wvar,
            theory.wexpr.wconst,
            theory.wexpr.wadd,
            theory.wexpr.wmul,
            theory.wexpr.wexpr_rec,
            theory.wexpr.weval,
            theory.wexpr.wderiv,
            theory.wexpr.wderiv_const_is_zero,
            theory.wexpr.eval_add_zero_right,
        ] {
            assert!(registry.is_declared(name));
            assert!(
                !registry.is_axiom(name),
                "every Workerman fact is proved, never assumed"
            );
        }
    }

    fn numeral(arena: &mut TermArena, prelude: &Prelude, n: u32) -> vf_core::TermId {
        let mut t = arena.const_(prelude.zero);
        let succ_c = arena.const_(prelude.succ);
        for _ in 0..n {
            t = arena.app(succ_c, t);
        }
        t
    }

    fn def_eq(
        arena: &mut TermArena,
        registry: &Registry,
        a: vf_core::TermId,
        b: vf_core::TermId,
        fuel: &mut u64,
    ) -> bool {
        vf_kernel::definitional_equal(arena, registry, a, b, fuel).unwrap()
    }

    #[test]
    fn add_and_mul_compute_correctly() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let theory = build_workerman_calculus(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        let add_c = arena.const_(theory.arith.add);
        let mul_c = arena.const_(theory.arith.mul);
        let two = numeral(&mut arena, &prelude, 2);
        let three = numeral(&mut arena, &prelude, 3);
        let five = numeral(&mut arena, &prelude, 5);
        let six = numeral(&mut arena, &prelude, 6);

        let sum = {
            let t = arena.app(add_c, two);
            arena.app(t, three)
        };
        assert!(def_eq(&mut arena, &registry, sum, five, &mut fuel));

        let product = {
            let t = arena.app(mul_c, two);
            arena.app(t, three)
        };
        assert!(def_eq(&mut arena, &registry, product, six, &mut fuel));
    }

    #[test]
    fn wderiv_follows_the_sum_and_product_rules_on_a_concrete_expression() {
        // e := wmul(wvar, wvar)  ("x * x")
        // wderiv(e) should compute to wadd(wmul(wconst 1, wvar), wmul(wvar, wconst 1))
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let theory = build_workerman_calculus(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        let wvar_c = arena.const_(theory.wexpr.wvar);
        let wmul_c = arena.const_(theory.wexpr.wmul);
        let wadd_c = arena.const_(theory.wexpr.wadd);
        let wconst_c = arena.const_(theory.wexpr.wconst);
        let wderiv_c = arena.const_(theory.wexpr.wderiv);
        let one = numeral(&mut arena, &prelude, 1);

        let e = {
            let t = arena.app(wmul_c, wvar_c);
            arena.app(t, wvar_c)
        };
        let deriv_e = arena.app(wderiv_c, e);

        let one_c = arena.app(wconst_c, one);
        let left = {
            let t = arena.app(wmul_c, one_c);
            arena.app(t, wvar_c)
        };
        let right = {
            let t = arena.app(wmul_c, wvar_c);
            arena.app(t, one_c)
        };
        let expected = {
            let t = arena.app(wadd_c, left);
            arena.app(t, right)
        };

        assert!(def_eq(&mut arena, &registry, deriv_e, expected, &mut fuel));
    }

    #[test]
    fn weval_computes_a_concrete_expression() {
        // e := wadd(wvar, wconst 10); weval(e, 5) should be 15.
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let theory = build_workerman_calculus(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        let wvar_c = arena.const_(theory.wexpr.wvar);
        let wadd_c = arena.const_(theory.wexpr.wadd);
        let wconst_c = arena.const_(theory.wexpr.wconst);
        let weval_c = arena.const_(theory.wexpr.weval);
        let ten = numeral(&mut arena, &prelude, 10);
        let five = numeral(&mut arena, &prelude, 5);
        let fifteen = numeral(&mut arena, &prelude, 15);

        let e = {
            let t = arena.app(wadd_c, wvar_c);
            let const_ten = arena.app(wconst_c, ten);
            arena.app(t, const_ten)
        };
        let result = {
            let t = arena.app(weval_c, e);
            arena.app(t, five)
        };
        assert!(def_eq(&mut arena, &registry, result, fifteen, &mut fuel));
    }
}
