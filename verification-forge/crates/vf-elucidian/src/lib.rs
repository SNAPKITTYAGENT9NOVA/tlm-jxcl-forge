// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Elucidian Algebra.
//!
//! ## This is not an established mathematical discipline
//!
//! "Elucidian Algebra" is a name invented for this project. It is not
//! a citation of, or a claim to formalize, any recognized branch of
//! mathematics -- not group theory, not lattice theory, not Boolean
//! algebra, even though its one concrete model happens to be built on
//! `Bool`. It is a small, deliberately narrow structure (one carrier,
//! two operations, three specific laws) picked to demonstrate
//! `vf-algebra`'s generic law-statement machinery and `vf-kernel`'s
//! ability to check a genuine inductive proof (not just a
//! computation) end to end. Nothing here should be read as a
//! contribution to, or a claim about, mathematics as practiced outside
//! this codebase.
//!
//! ## The theory
//!
//! A carrier `E` (concretely `Bool`, from `vf-axioms`'s prelude) with:
//! - `diverge : E -> E -> E`, defined as left-projection
//!   (`diverge(x, y) := x`) -- deliberately unusual: most named binary
//!   operations are not simply "return the first argument".
//! - `reflect : E -> E`, defined as `Bool`'s negation.
//!
//! satisfying three laws, each a genuine `vf-kernel`-checked theorem
//! (never an axiom -- there is nothing here that couldn't be proved
//! outright, so nothing here is postulated):
//! - `diverge` is idempotent: `diverge(x, x) = x`.
//! - `diverge` is left-absorbing: `diverge(diverge(x, y), z) = diverge(x, z)`.
//! - `reflect` is involutive: `reflect(reflect(x)) = x`.
//!
//! The first two hold by pure computation (`diverge` always reduces
//! away its second argument, so both sides beta-reduce to the same
//! term for an arbitrary `x`) and are proved with nothing more than
//! `refl`. The third does *not* hold by computation alone for an
//! abstract `x` (negation is only computable once `x` is a concrete
//! `true`/`false`), so its proof is a genuine case split via
//! `Bool_ind`'s dependent elimination into `Prop` (not `Bool_rec`,
//! which can only eliminate into `Type 0` -- see this module's doc
//! comment on `build_elucidian_model`) -- the one proof in this crate
//! that is actually using induction, not just evaluation.
#![forbid(unsafe_code)]

use vf_axioms::{Prelude, Registry, RegistryError};
use vf_core::{ArenaError, Binder, Interner, Symbol, TermArena, TermId};

fn lam1(
    arena: &mut TermArena,
    interner: &mut Interner,
    hint: &str,
    domain: TermId,
    build_body: impl FnOnce(&mut TermArena, &mut Interner, TermId) -> Result<TermId, ArenaError>,
) -> Result<TermId, ArenaError> {
    let sym = interner.intern(hint);
    let placeholder = arena.free_var(sym);
    let body_open = build_body(arena, interner, placeholder)?;
    let body_closed = arena.close_at(body_open, 0, sym)?;
    Ok(arena.lam(Binder(sym), domain, body_closed))
}

fn arrow(
    arena: &mut TermArena,
    interner: &mut Interner,
    domain: TermId,
    codomain: TermId,
) -> TermId {
    arena.pi(Binder(interner.intern("_")), domain, codomain)
}

/// The Elucidian Algebra's concrete model and its three proved laws.
#[derive(Debug, Clone, Copy)]
pub struct ElucidianModel {
    /// The carrier -- concretely `Bool` (see this crate's doc comment
    /// for why "Elucidian Algebra" is nonetheless not Boolean algebra).
    pub carrier: Symbol,
    pub diverge: Symbol,
    pub reflect: Symbol,
    pub diverge_idempotent: Symbol,
    pub diverge_left_absorbing: Symbol,
    pub reflect_involutive: Symbol,
}

/// Define `diverge`/`reflect` on `prelude.bool_` and prove all three
/// laws, registering everything into `registry`. `prelude` must
/// already be registered (see `vf_axioms::register_prelude`) --
/// `Bool`, `Bool_rec`, `Bool_ind`, and `refl` are all used here.
pub fn build_elucidian_model(
    registry: &mut Registry,
    prelude: &Prelude,
    arena: &mut TermArena,
    interner: &mut Interner,
    fuel: &mut u64,
) -> Result<ElucidianModel, RegistryError> {
    let bool_c = arena.const_(prelude.bool_);
    let true_c = arena.const_(prelude.true_);
    let false_c = arena.const_(prelude.false_);
    let bool_rec_c = arena.const_(prelude.bool_rec);
    let bool_ind_c = arena.const_(prelude.bool_ind);
    let refl_c = arena.const_(prelude.refl);

    // ---- diverge : Bool -> Bool -> Bool := fun x y => x ----
    let diverge_name = interner.intern("diverge");
    let diverge_ty = {
        let inner = arrow(arena, interner, bool_c, bool_c);
        arrow(arena, interner, bool_c, inner)
    };
    let diverge_val = lam1(arena, interner, "x", bool_c, |arena, interner, x| {
        lam1(
            arena,
            interner,
            "y",
            bool_c,
            move |_arena, _interner, _y| Ok(x),
        )
    })?;
    registry.declare_definition(arena, interner, diverge_name, diverge_ty, diverge_val, fuel)?;
    let diverge_c = arena.const_(diverge_name);

    // ---- reflect : Bool -> Bool := fun x => Bool_rec (fun _ => Bool) false true x ----
    let reflect_name = interner.intern("reflect");
    let reflect_ty = arrow(arena, interner, bool_c, bool_c);
    let reflect_val = lam1(arena, interner, "x", bool_c, |arena, interner, x| {
        let motive = arena.lam(Binder(interner.intern("_")), bool_c, bool_c);
        let t = arena.app(bool_rec_c, motive);
        let t = arena.app(t, false_c);
        let t = arena.app(t, true_c);
        let _ = interner;
        Ok(arena.app(t, x))
    })?;
    registry.declare_definition(arena, interner, reflect_name, reflect_ty, reflect_val, fuel)?;
    let reflect_c = arena.const_(reflect_name);

    // ---- diverge is idempotent: diverge(x,x) = x, by pure computation ----
    let idempotent_name = interner.intern("diverge_idempotent");
    let idempotent_stmt = vf_algebra::idempotent_binary(arena, interner, bool_c, diverge_c)?;
    let idempotent_proof = lam1(arena, interner, "x", bool_c, |arena, _interner, x| {
        let t = arena.app(refl_c, bool_c);
        Ok(arena.app(t, x))
    })?;
    registry.declare_theorem(
        arena,
        interner,
        idempotent_name,
        idempotent_stmt,
        idempotent_proof,
        fuel,
    )?;

    // ---- diverge is left-absorbing: diverge(diverge(x,y),z) = diverge(x,z), by pure computation ----
    let left_absorbing_name = interner.intern("diverge_left_absorbing");
    let left_absorbing_stmt = vf_algebra::left_absorbing(arena, interner, bool_c, diverge_c)?;
    let left_absorbing_proof = lam1(arena, interner, "x", bool_c, |arena, interner, x| {
        lam1(arena, interner, "y", bool_c, move |arena, interner, _y| {
            lam1(arena, interner, "z", bool_c, move |arena, _interner, _z| {
                let t = arena.app(refl_c, bool_c);
                Ok(arena.app(t, x))
            })
        })
    })?;
    registry.declare_theorem(
        arena,
        interner,
        left_absorbing_name,
        left_absorbing_stmt,
        left_absorbing_proof,
        fuel,
    )?;

    // ---- reflect is involutive: reflect(reflect(x)) = x, by case split ----
    // Not provable by refl alone: `reflect` only computes once its
    // argument is a concrete `true`/`false`, and `x` here is abstract.
    // The proof is genuine induction: eliminate `x` into `Prop` via
    // `Bool_ind` (not `Bool_rec` -- proving a proposition needs a
    // Prop-valued motive, which `Bool_rec`'s Type-0-valued motive
    // can't provide in a kernel with no universe polymorphism; see
    // vf-axioms::prelude's registration of `Bool_ind`), proving each
    // of the two concrete cases (which DO reduce fully) by refl.
    let involutive_name = interner.intern("reflect_involutive");
    let involutive_stmt = vf_algebra::involutive(arena, interner, bool_c, reflect_c)?;
    let involutive_proof = lam1(arena, interner, "x", bool_c, |arena, interner, x| {
        // motive := fun (x : Bool) => Eq(Bool, reflect(reflect(x)), x)
        let motive = lam1(arena, interner, "x", bool_c, move |arena, _interner, mx| {
            let rmx = arena.app(reflect_c, mx);
            let rrmx = arena.app(reflect_c, rmx);
            Ok(arena.eq(bool_c, rrmx, mx))
        })?;
        let case_true = {
            let t = arena.app(refl_c, bool_c);
            arena.app(t, true_c)
        };
        let case_false = {
            let t = arena.app(refl_c, bool_c);
            arena.app(t, false_c)
        };
        let t = arena.app(bool_ind_c, motive);
        let t = arena.app(t, case_true);
        let t = arena.app(t, case_false);
        let _ = interner;
        Ok(arena.app(t, x))
    })?;
    registry.declare_theorem(
        arena,
        interner,
        involutive_name,
        involutive_stmt,
        involutive_proof,
        fuel,
    )?;

    Ok(ElucidianModel {
        carrier: prelude.bool_,
        diverge: diverge_name,
        reflect: reflect_name,
        diverge_idempotent: idempotent_name,
        diverge_left_absorbing: left_absorbing_name,
        reflect_involutive: involutive_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_axioms::AxiomPolicy;
    use vf_reducer::DEFAULT_FUEL;

    #[test]
    fn the_elucidian_model_and_all_three_laws_check_against_the_kernel() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut registry = Registry::new(AxiomPolicy::NoAxioms);
        let mut fuel = DEFAULT_FUEL;
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let model = build_elucidian_model(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        for name in [
            model.diverge,
            model.reflect,
            model.diverge_idempotent,
            model.diverge_left_absorbing,
            model.reflect_involutive,
        ] {
            assert!(registry.is_declared(name));
            assert!(
                !registry.is_axiom(name),
                "every Elucidian fact is proved, never assumed"
            );
        }
    }

    #[test]
    fn diverge_really_is_left_projection() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut registry = Registry::new(AxiomPolicy::NoAxioms);
        let mut fuel = DEFAULT_FUEL;
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let model = build_elucidian_model(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        let diverge_c = arena.const_(model.diverge);
        let true_c = arena.const_(prelude.true_);
        let false_c = arena.const_(prelude.false_);
        let applied = {
            let t = arena.app(diverge_c, true_c);
            arena.app(t, false_c)
        };
        assert!(vf_kernel_style_eq(
            &mut arena, &registry, applied, true_c, &mut fuel
        ));
    }

    #[test]
    fn reflect_really_is_negation() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut registry = Registry::new(AxiomPolicy::NoAxioms);
        let mut fuel = DEFAULT_FUEL;
        let prelude =
            vf_axioms::register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel)
                .unwrap();
        let model = build_elucidian_model(
            &mut registry,
            &prelude,
            &mut arena,
            &mut interner,
            &mut fuel,
        )
        .unwrap();

        let reflect_c = arena.const_(model.reflect);
        let true_c = arena.const_(prelude.true_);
        let false_c = arena.const_(prelude.false_);
        let reflect_true = arena.app(reflect_c, true_c);
        assert!(vf_kernel_style_eq(
            &mut arena,
            &registry,
            reflect_true,
            false_c,
            &mut fuel
        ));
        let reflect_false = arena.app(reflect_c, false_c);
        assert!(vf_kernel_style_eq(
            &mut arena,
            &registry,
            reflect_false,
            true_c,
            &mut fuel
        ));
    }

    fn vf_kernel_style_eq(
        arena: &mut TermArena,
        registry: &Registry,
        a: TermId,
        b: TermId,
        fuel: &mut u64,
    ) -> bool {
        // Registry implements vf_kernel::KernelEnv; using
        // definitional_equal directly here avoids adding a vf-kernel
        // dependency to this crate's main (non-test) code just for
        // this one assertion helper.
        vf_kernel::definitional_equal(arena, registry, a, b, fuel).unwrap()
    }
}
