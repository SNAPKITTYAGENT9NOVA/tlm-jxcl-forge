// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `WExpr`: Workerman's Calculus's expression type, plus `weval`
//! (evaluation) and `wderiv` ("the Workerman derivative" -- see this
//! crate's top-level doc comment for why that name is in scare quotes).
use crate::arith::Arith;
use crate::helpers::{app2, app4, arrow, dep_pi, lam1};
use vf_axioms::{Prelude, Registry, RegistryError};
use vf_core::{Binder, Interner, Sort, Symbol, TermArena, TermId};
use vf_reducer::{ConstructorSpec, RecursorSpec};

/// `WExpr`'s type former, constructors, recursor, and the two derived
/// functions built on top of it.
#[derive(Debug, Clone, Copy)]
pub struct WExpr {
    pub wexpr: Symbol,
    pub wvar: Symbol,
    pub wconst: Symbol,
    pub wadd: Symbol,
    pub wmul: Symbol,
    pub wexpr_rec: Symbol,
    /// `weval : WExpr -> Nat -> Nat`.
    pub weval: Symbol,
    /// `wderiv : WExpr -> WExpr`.
    pub wderiv: Symbol,
    /// `Pi (n : Nat), Eq(WExpr, wderiv(wconst n), wconst zero)`.
    pub wderiv_const_is_zero: Symbol,
    /// `Pi (e : WExpr) (x : Nat), Eq(Nat, weval(wadd(e, wconst zero), x), weval(e, x))`.
    pub eval_add_zero_right: Symbol,
}

#[allow(clippy::too_many_lines)]
pub fn build_wexpr(
    registry: &mut Registry,
    prelude: &Prelude,
    arith: &Arith,
    arena: &mut TermArena,
    interner: &mut Interner,
    fuel: &mut u64,
) -> Result<WExpr, RegistryError> {
    let type0 = arena.sort(Sort::Type(0));
    let nat_c = arena.const_(prelude.nat);
    let zero_c = arena.const_(prelude.zero);
    let refl_c = arena.const_(prelude.refl);
    let add_c = arena.const_(arith.add);

    // ---- WExpr : Type 0 ----
    let wexpr = interner.intern("WExpr");
    registry.declare_builtin(arena, interner, wexpr, type0, fuel)?;
    let wexpr_c = arena.const_(wexpr);

    // ---- wvar : WExpr ---- (the sole variable "x")
    let wvar = interner.intern("wvar");
    registry.declare_builtin(arena, interner, wvar, wexpr_c, fuel)?;
    let wvar_c = arena.const_(wvar);

    // ---- wconst : Nat -> WExpr ----
    let wconst = interner.intern("wconst");
    let wconst_ty = arrow(arena, interner, nat_c, wexpr_c);
    registry.declare_builtin(arena, interner, wconst, wconst_ty, fuel)?;
    let wconst_c = arena.const_(wconst);

    // ---- wadd, wmul : WExpr -> WExpr -> WExpr ----
    let wadd = interner.intern("wadd");
    let wadd_ty = {
        let inner = arrow(arena, interner, wexpr_c, wexpr_c);
        arrow(arena, interner, wexpr_c, inner)
    };
    registry.declare_builtin(arena, interner, wadd, wadd_ty, fuel)?;
    let wadd_c = arena.const_(wadd);

    let wmul = interner.intern("wmul");
    let wmul_ty = {
        let inner = arrow(arena, interner, wexpr_c, wexpr_c);
        arrow(arena, interner, wexpr_c, inner)
    };
    registry.declare_builtin(arena, interner, wmul, wmul_ty, fuel)?;
    let wmul_c = arena.const_(wmul);

    // ---- WExpr_rec : Pi (P : WExpr -> Type 0),
    //        P wvar ->
    //        (Pi (n : Nat), P (wconst n)) ->
    //        (Pi (a b : WExpr), P a -> P b -> P (wadd a b)) ->
    //        (Pi (a b : WExpr), P a -> P b -> P (wmul a b)) ->
    //        Pi (e : WExpr), P e
    let wexpr_rec = interner.intern("WExpr_rec");
    let wexpr_motive_ty = arrow(arena, interner, wexpr_c, type0);
    let wexpr_rec_ty = dep_pi(
        arena,
        interner,
        "P",
        wexpr_motive_ty,
        |arena, interner, p| {
            let p_wvar = arena.app(p, wvar_c);
            dep_pi(
                arena,
                interner,
                "pv",
                p_wvar,
                move |arena, interner, _pv| {
                    let case_const_ty =
                        dep_pi(arena, interner, "n", nat_c, move |arena, _interner, n| {
                            let wconst_n = arena.app(wconst_c, n);
                            Ok(arena.app(p, wconst_n))
                        })?;
                    dep_pi(
                        arena,
                        interner,
                        "pc",
                        case_const_ty,
                        move |arena, interner, _pc| {
                            let binop_case_ty =
                                |arena: &mut TermArena, interner: &mut Interner, ctor: TermId| {
                                    dep_pi(
                                        arena,
                                        interner,
                                        "a",
                                        wexpr_c,
                                        move |arena, interner, a| {
                                            let p_a = arena.app(p, a);
                                            dep_pi(
                                                arena,
                                                interner,
                                                "ih_a",
                                                p_a,
                                                move |arena, interner, _ih_a| {
                                                    dep_pi(
                                                        arena,
                                                        interner,
                                                        "b",
                                                        wexpr_c,
                                                        move |arena, interner, b| {
                                                            let p_b = arena.app(p, b);
                                                            dep_pi(
                                                                arena,
                                                                interner,
                                                                "ih_b",
                                                                p_b,
                                                                move |arena, _interner, _ih_b| {
                                                                    let ctor_a_b =
                                                                        app2(arena, ctor, a, b);
                                                                    Ok(arena.app(p, ctor_a_b))
                                                                },
                                                            )
                                                        },
                                                    )
                                                },
                                            )
                                        },
                                    )
                                };
                            let add_case_ty = binop_case_ty(arena, interner, wadd_c)?;
                            dep_pi(
                                arena,
                                interner,
                                "pa",
                                add_case_ty,
                                move |arena, interner, _pa| {
                                    let mul_case_ty = binop_case_ty(arena, interner, wmul_c)?;
                                    dep_pi(
                                        arena,
                                        interner,
                                        "pm",
                                        mul_case_ty,
                                        move |arena, interner, _pm| {
                                            dep_pi(
                                                arena,
                                                interner,
                                                "e",
                                                wexpr_c,
                                                move |arena, _interner, e| Ok(arena.app(p, e)),
                                            )
                                        },
                                    )
                                },
                            )
                        },
                    )
                },
            )
        },
    )?;
    let wexpr_rec_spec = RecursorSpec {
        arity: 6, // P, pv, pc, pa, pm, e(major)
        case_positions: vec![1, 2, 3, 4],
        constructor_names: vec![wvar, wconst, wadd, wmul],
        constructors: vec![
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![],
            },
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![false],
            },
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![true, true],
            },
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![true, true],
            },
        ],
    };
    registry.declare_recursor(
        arena,
        interner,
        wexpr_rec,
        wexpr_rec_ty,
        wexpr_rec_spec,
        fuel,
    )?;
    let wexpr_rec_c = arena.const_(wexpr_rec);

    // ---- weval : WExpr -> Nat -> Nat ----
    // weval(wvar) = fun x => x
    // weval(wconst n) = fun x => n
    // weval(wadd a b) = fun x => add (weval a x) (weval b x)
    // weval(wmul a b) = fun x => mul (weval a x) (weval b x)
    let mul_c = arena.const_(arith.mul);
    let weval = interner.intern("weval");
    let weval_ty = {
        let inner = arrow(arena, interner, nat_c, nat_c);
        arrow(arena, interner, wexpr_c, inner)
    };
    let nat_to_nat = arrow(arena, interner, nat_c, nat_c);
    let weval_motive = arena.lam(Binder(interner.intern("_")), wexpr_c, nat_to_nat);
    let case_var_eval = {
        // fun (x:Nat) => x
        let x = arena.bound_var(0);
        arena.lam(Binder(interner.intern("x")), nat_c, x)
    };
    let case_const_eval = lam1(arena, interner, "n", nat_c, |arena, interner, n| {
        // fun (n:Nat) => fun (_x:Nat) => n
        Ok(arena.lam(Binder(interner.intern("_x")), nat_c, n))
    })?;
    // Each of `ih_a`/`ih_b` has type `P a`/`P b`, which beta-reduces to
    // `nat_to_nat` (`weval`'s motive ignores its `WExpr` argument) --
    // that reduced type, not the motive function itself, is what these
    // lambdas' own parameter annotations must be.
    let case_add_eval = lam1(arena, interner, "a", wexpr_c, |arena, interner, _a| {
        lam1(
            arena,
            interner,
            "ih_a",
            nat_to_nat,
            move |arena, interner, ih_a| {
                lam1(arena, interner, "b", wexpr_c, move |arena, interner, _b| {
                    lam1(
                        arena,
                        interner,
                        "ih_b",
                        nat_to_nat,
                        move |arena, interner, ih_b| {
                            lam1(arena, interner, "x", nat_c, move |arena, _interner, x| {
                                let ih_a_x = arena.app(ih_a, x);
                                let ih_b_x = arena.app(ih_b, x);
                                Ok(app2(arena, add_c, ih_a_x, ih_b_x))
                            })
                        },
                    )
                })
            },
        )
    })?;
    let case_mul_eval = lam1(arena, interner, "a", wexpr_c, |arena, interner, _a| {
        lam1(
            arena,
            interner,
            "ih_a",
            nat_to_nat,
            move |arena, interner, ih_a| {
                lam1(arena, interner, "b", wexpr_c, move |arena, interner, _b| {
                    lam1(
                        arena,
                        interner,
                        "ih_b",
                        nat_to_nat,
                        move |arena, interner, ih_b| {
                            lam1(arena, interner, "x", nat_c, move |arena, _interner, x| {
                                let ih_a_x = arena.app(ih_a, x);
                                let ih_b_x = arena.app(ih_b, x);
                                Ok(app2(arena, mul_c, ih_a_x, ih_b_x))
                            })
                        },
                    )
                })
            },
        )
    })?;
    let weval_val = lam1(arena, interner, "e", wexpr_c, move |arena, _interner, e| {
        let t = app4(
            arena,
            wexpr_rec_c,
            weval_motive,
            case_var_eval,
            case_const_eval,
            case_add_eval,
        );
        let t = arena.app(t, case_mul_eval);
        Ok(arena.app(t, e))
    })?;
    registry.declare_definition(arena, interner, weval, weval_ty, weval_val, fuel)?;
    let weval_c = arena.const_(weval);

    // ---- wderiv : WExpr -> WExpr ----
    // wderiv(wvar) = wconst 1
    // wderiv(wconst n) = wconst 0
    // wderiv(wadd a b) = wadd (wderiv a) (wderiv b)                          -- sum rule
    // wderiv(wmul a b) = wadd (wmul (wderiv a) b) (wmul a (wderiv b))        -- product rule
    let one = {
        let succ_c = arena.const_(prelude.succ);
        arena.app(succ_c, zero_c)
    };
    let wderiv_motive = arena.lam(Binder(interner.intern("_")), wexpr_c, wexpr_c);
    let case_var_deriv = arena.app(wconst_c, one);
    let case_const_deriv = lam1(arena, interner, "n", nat_c, |arena, _interner, _n| {
        Ok(arena.app(wconst_c, zero_c))
    })?;
    let case_add_deriv = lam1(arena, interner, "a", wexpr_c, |arena, interner, _a| {
        lam1(
            arena,
            interner,
            "ih_a",
            wexpr_c,
            move |arena, interner, ih_a| {
                lam1(arena, interner, "b", wexpr_c, move |arena, interner, _b| {
                    lam1(
                        arena,
                        interner,
                        "ih_b",
                        wexpr_c,
                        move |arena, _interner, ih_b| Ok(app2(arena, wadd_c, ih_a, ih_b)),
                    )
                })
            },
        )
    })?;
    let case_mul_deriv = lam1(arena, interner, "a", wexpr_c, |arena, interner, a| {
        lam1(
            arena,
            interner,
            "ih_a",
            wexpr_c,
            move |arena, interner, ih_a| {
                lam1(arena, interner, "b", wexpr_c, move |arena, interner, b| {
                    lam1(
                        arena,
                        interner,
                        "ih_b",
                        wexpr_c,
                        move |arena, _interner, ih_b| {
                            let left = app2(arena, wmul_c, ih_a, b);
                            let right = app2(arena, wmul_c, a, ih_b);
                            Ok(app2(arena, wadd_c, left, right))
                        },
                    )
                })
            },
        )
    })?;
    let wderiv_name = interner.intern("wderiv");
    let wderiv_ty = arrow(arena, interner, wexpr_c, wexpr_c);
    let wderiv_val = lam1(arena, interner, "e", wexpr_c, move |arena, _interner, e| {
        let t = app4(
            arena,
            wexpr_rec_c,
            wderiv_motive,
            case_var_deriv,
            case_const_deriv,
            case_add_deriv,
        );
        let t = arena.app(t, case_mul_deriv);
        Ok(arena.app(t, e))
    })?;
    registry.declare_definition(arena, interner, wderiv_name, wderiv_ty, wderiv_val, fuel)?;
    let wderiv_c = arena.const_(wderiv_name);

    // ---- wderiv_const_is_zero : Pi (n : Nat), Eq(WExpr, wderiv(wconst n), wconst zero) ----
    // Holds by pure computation: `wconst n` is already constructor-headed
    // regardless of whether `n` itself is concrete, so WExpr_rec's
    // wconst case fires unconditionally.
    let wderiv_const_is_zero = interner.intern("wderiv_const_is_zero");
    let wderiv_const_is_zero_stmt = dep_pi(arena, interner, "n", nat_c, |arena, interner, n| {
        let wconst_n = arena.app(wconst_c, n);
        let wderiv_wconst_n = arena.app(wderiv_c, wconst_n);
        let wconst_zero = arena.app(wconst_c, zero_c);
        let _ = interner;
        Ok(arena.eq(wexpr_c, wderiv_wconst_n, wconst_zero))
    })?;
    let wderiv_const_is_zero_proof = lam1(arena, interner, "n", nat_c, |arena, _interner, _n| {
        let wconst_zero = arena.app(wconst_c, zero_c);
        let t = arena.app(refl_c, wexpr_c);
        Ok(arena.app(t, wconst_zero))
    })?;
    registry.declare_theorem(
        arena,
        interner,
        wderiv_const_is_zero,
        wderiv_const_is_zero_stmt,
        wderiv_const_is_zero_proof,
        fuel,
    )?;

    // ---- eval_add_zero_right : Pi (e : WExpr) (x : Nat),
    //        Eq(Nat, weval(wadd(e, wconst zero), x), weval(e, x)) ----
    // weval(wadd(e,wconst 0), x) normalizes to add(weval(e,x), 0)
    // (wadd/wconst-0 are both constructor-headed regardless of `e`),
    // which `add_zero_right` applied to `weval(e,x)` proves equal to
    // `weval(e,x)` directly -- no WExpr-level induction needed, only
    // reusing the already-proven Nat lemma.
    let eval_add_zero_right = interner.intern("eval_add_zero_right");
    let eval_add_zero_right_stmt = dep_pi(arena, interner, "e", wexpr_c, |arena, interner, e| {
        dep_pi(arena, interner, "x", nat_c, move |arena, _interner, x| {
            let wconst_zero = arena.app(wconst_c, zero_c);
            let e_plus_zero = app2(arena, wadd_c, e, wconst_zero);
            let lhs = app2(arena, weval_c, e_plus_zero, x);
            let rhs = app2(arena, weval_c, e, x);
            Ok(arena.eq(nat_c, lhs, rhs))
        })
    })?;
    let add_zero_right_c = arena.const_(arith.add_zero_right);
    let eval_add_zero_right_proof = lam1(arena, interner, "e", wexpr_c, |arena, interner, e| {
        lam1(arena, interner, "x", nat_c, move |arena, _interner, x| {
            let eval_e_x = app2(arena, weval_c, e, x);
            Ok(arena.app(add_zero_right_c, eval_e_x))
        })
    })?;
    registry.declare_theorem(
        arena,
        interner,
        eval_add_zero_right,
        eval_add_zero_right_stmt,
        eval_add_zero_right_proof,
        fuel,
    )?;

    Ok(WExpr {
        wexpr,
        wvar,
        wconst,
        wadd,
        wmul,
        wexpr_rec,
        weval,
        wderiv: wderiv_name,
        wderiv_const_is_zero,
        eval_add_zero_right,
    })
}
