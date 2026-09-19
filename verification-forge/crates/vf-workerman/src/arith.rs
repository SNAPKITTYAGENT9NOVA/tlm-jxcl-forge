//! `Nat` arithmetic (`add`/`mul`) and the one lemma about it this
//! crate needs (`add_zero_right`), defined and proved locally here
//! rather than added to `vf-axioms`'s prelude -- the prelude's scope
//! is the fixed set of type formers/constructors/recursors, not
//! arbitrary derived arithmetic, and this crate is exactly where
//! `add`/`mul` are actually needed (by `weval`, for `wadd`/`wmul`).
use crate::helpers::{app2, arrow, dep_pi, lam1};
use vf_axioms::{Prelude, Registry, RegistryError};
use vf_core::{Binder, Interner, Symbol, TermArena};

/// `add`, `mul`, and the proof that `add` has a right identity.
#[derive(Debug, Clone, Copy)]
pub struct Arith {
    pub add: Symbol,
    pub mul: Symbol,
    /// `Pi (n : Nat), Eq(Nat, add(n, zero), n)`.
    pub add_zero_right: Symbol,
}

pub fn build_arith(
    registry: &mut Registry,
    prelude: &Prelude,
    arena: &mut TermArena,
    interner: &mut Interner,
    fuel: &mut u64,
) -> Result<Arith, RegistryError> {
    let nat_c = arena.const_(prelude.nat);
    let zero_c = arena.const_(prelude.zero);
    let succ_c = arena.const_(prelude.succ);
    let nat_rec_c = arena.const_(prelude.nat_rec);
    let nat_ind_c = arena.const_(prelude.nat_ind);
    let refl_c = arena.const_(prelude.refl);
    let ap_c = arena.const_(prelude.ap);

    // ---- add : Nat -> Nat -> Nat := fun m n => Nat_rec (fun _ => Nat) n (fun _ ih => succ ih) m ----
    // Recursion on the first argument: add(zero, n) = n; add(succ m, n) = succ(add(m, n)).
    let add_name = interner.intern("add");
    let add_ty = {
        let inner = arrow(arena, interner, nat_c, nat_c);
        arrow(arena, interner, nat_c, inner)
    };
    let motive = arena.lam(Binder(interner.intern("_")), nat_c, nat_c);
    let add_val = lam1(arena, interner, "m", nat_c, move |arena, interner, m| {
        lam1(arena, interner, "n", nat_c, move |arena, interner, n| {
            let case_succ = {
                let ih = arena.bound_var(0);
                let succ_ih = arena.app(succ_c, ih);
                let inner_lam = arena.lam(Binder(interner.intern("ih")), nat_c, succ_ih);
                arena.lam(Binder(interner.intern("_pred")), nat_c, inner_lam)
            };
            let t = arena.app(nat_rec_c, motive);
            let t = arena.app(t, n);
            let t = arena.app(t, case_succ);
            Ok(arena.app(t, m))
        })
    })?;
    registry.declare_definition(arena, interner, add_name, add_ty, add_val, fuel)?;
    let add_c = arena.const_(add_name);

    // ---- mul : Nat -> Nat -> Nat := fun m n => Nat_rec (fun _ => Nat) zero (fun _ ih => add ih n) m ----
    let mul_name = interner.intern("mul");
    let mul_ty = {
        let inner = arrow(arena, interner, nat_c, nat_c);
        arrow(arena, interner, nat_c, inner)
    };
    let mul_val = lam1(arena, interner, "m", nat_c, move |arena, interner, m| {
        lam1(arena, interner, "n", nat_c, move |arena, interner, n| {
            let case_succ = {
                let ih = arena.bound_var(0);
                let add_ih_n = app2(arena, add_c, ih, n);
                let inner_lam = arena.lam(Binder(interner.intern("ih")), nat_c, add_ih_n);
                arena.lam(Binder(interner.intern("_pred")), nat_c, inner_lam)
            };
            let t = arena.app(nat_rec_c, motive);
            let t = arena.app(t, zero_c);
            let t = arena.app(t, case_succ);
            Ok(arena.app(t, m))
        })
    })?;
    registry.declare_definition(arena, interner, mul_name, mul_ty, mul_val, fuel)?;

    // ---- add_zero_right : Pi (n : Nat), Eq(Nat, add(n, zero), n) ----
    // The classic first induction exercise: add's own definition
    // (recursion on the *first* argument) makes `add(zero, n) = n`
    // hold by computation alone, but `add(n, zero) = n` needs
    // induction on `n` -- the base case computes directly, the step
    // case needs `ap` to lift the inductive hypothesis under `succ`.
    let add_zero_right_name = interner.intern("add_zero_right");
    let add_zero_right_stmt = dep_pi(arena, interner, "n", nat_c, move |arena, _interner, n| {
        let add_n_zero = app2(arena, add_c, n, zero_c);
        Ok(arena.eq(nat_c, add_n_zero, n))
    })?;
    let add_zero_right_proof = {
        // motive := fun (n : Nat) => Eq(Nat, add(n, zero), n)
        let motive = lam1(arena, interner, "n", nat_c, move |arena, _interner, n| {
            let add_n_zero = app2(arena, add_c, n, zero_c);
            Ok(arena.eq(nat_c, add_n_zero, n))
        })?;
        let case_zero = {
            // Eq(Nat, add(zero, zero), zero) -- add(zero,zero) computes to zero directly.
            let t = arena.app(refl_c, nat_c);
            arena.app(t, zero_c)
        };
        let case_succ = lam1(
            arena,
            interner,
            "n",
            nat_c,
            move |arena, interner, n_pred| {
                let add_n_zero = app2(arena, add_c, n_pred, zero_c);
                let ih_ty = arena.eq(nat_c, add_n_zero, n_pred);
                lam1(arena, interner, "ih", ih_ty, move |arena, _interner, ih| {
                    // ap Nat Nat succ (add n_pred zero) n_pred ih
                    //   : Eq(Nat, succ(add(n_pred,zero)), succ(n_pred))
                    let t = arena.app(ap_c, nat_c);
                    let t = arena.app(t, nat_c);
                    let t = arena.app(t, succ_c);
                    let t = arena.app(t, add_n_zero);
                    let t = arena.app(t, n_pred);
                    Ok(arena.app(t, ih))
                })
            },
        )?;
        // fun (n : Nat) => Nat_ind motive case_zero case_succ n
        lam1(arena, interner, "n", nat_c, move |arena, _interner, n| {
            let t = arena.app(nat_ind_c, motive);
            let t = arena.app(t, case_zero);
            let t = arena.app(t, case_succ);
            Ok(arena.app(t, n))
        })?
    };
    registry.declare_theorem(
        arena,
        interner,
        add_zero_right_name,
        add_zero_right_stmt,
        add_zero_right_proof,
        fuel,
    )?;

    Ok(Arith {
        add: add_name,
        mul: mul_name,
        add_zero_right: add_zero_right_name,
    })
}
