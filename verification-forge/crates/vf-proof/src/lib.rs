//! Proof-term and theorem representation with explicit
//! axiom/definition dependency closures.
//!
//! ## Trust status
//!
//! Untrusted, like `vf-axioms`: this crate doesn't itself decide
//! whether anything is well-typed (that already happened when
//! `vf-axioms::Registry` accepted the declaration). What it adds is
//! *provenance*: given a declared name, [`dependency_closure`] answers
//! "what does this actually depend on", by walking the registry's
//! already-DAG-shaped reference graph rather than trusting a claim
//! about what was used. A bug here could misreport a dependency, but
//! it cannot make an unsound theorem type-check -- that gate is
//! entirely `vf-kernel`'s.
#![forbid(unsafe_code)]

mod closure;
mod error;
mod theorem;

pub use closure::{dependency_closure, DependencyClosure};
pub use error::ProofError;
pub use theorem::{checked_theorem, CheckedTheorem};

#[cfg(test)]
mod tests {
    use super::*;
    use vf_axioms::{AxiomPolicy, Registry};
    use vf_core::{Interner, Sort, TermArena};
    use vf_reducer::DEFAULT_FUEL;

    #[test]
    fn an_axiom_free_definition_has_an_empty_axiom_closure() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut reg = Registry::new(AxiomPolicy::NoAxioms);
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let name = interner.intern("my_type");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_definition(&mut arena, &mut interner, name, ty1, ty0, &mut fuel)
            .unwrap();

        let closure = dependency_closure(&arena, &reg, name).unwrap();
        assert!(closure.is_axiom_free());
        assert!(closure.definitions.is_empty());
    }

    #[test]
    fn a_definition_referencing_an_axiom_reports_it_in_the_closure() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let axiom_name = interner.intern("Base");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_axiom(
            &mut arena,
            &mut interner,
            axiom_name,
            ty0,
            "a base type",
            &mut fuel,
        )
        .unwrap();

        let const_ref = arena.const_(axiom_name);
        let alias_name = interner.intern("alias");
        reg.declare_definition(
            &mut arena,
            &mut interner,
            alias_name,
            ty0,
            const_ref,
            &mut fuel,
        )
        .unwrap();

        let closure = dependency_closure(&arena, &reg, alias_name).unwrap();
        assert!(!closure.is_axiom_free());
        assert!(closure.axioms.contains(&axiom_name));
        assert!(closure.definitions.is_empty());
    }

    #[test]
    fn a_transitive_axiom_dependency_is_still_reported() {
        // axiom Base : Type 0
        // def layer1 : Type 0 := Base
        // def layer2 : Type 0 := layer1
        // layer2's closure must contain Base even though layer2 never
        // references it directly.
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let mut fuel = DEFAULT_FUEL;

        let base = interner.intern("Base");
        reg.declare_axiom(&mut arena, &mut interner, base, ty0, "base", &mut fuel)
            .unwrap();

        let base_ref = arena.const_(base);
        let layer1 = interner.intern("layer1");
        reg.declare_definition(&mut arena, &mut interner, layer1, ty0, base_ref, &mut fuel)
            .unwrap();

        let layer1_ref = arena.const_(layer1);
        let layer2 = interner.intern("layer2");
        reg.declare_definition(
            &mut arena,
            &mut interner,
            layer2,
            ty0,
            layer1_ref,
            &mut fuel,
        )
        .unwrap();

        let closure = dependency_closure(&arena, &reg, layer2).unwrap();
        assert!(closure.axioms.contains(&base));
        assert!(closure.definitions.contains(&layer1));
        assert!(!closure.is_axiom_free());
    }

    #[test]
    fn an_axiom_s_own_closure_is_just_itself() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("Base");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_axiom(&mut arena, &mut interner, name, ty0, "base", &mut fuel)
            .unwrap();

        let closure = dependency_closure(&arena, &reg, name).unwrap();
        assert_eq!(closure.axioms.len(), 1);
        assert!(closure.axioms.contains(&name));
        assert!(closure.definitions.is_empty());
    }

    #[test]
    fn an_undeclared_root_is_rejected() {
        let arena = TermArena::new();
        let mut interner = Interner::new();
        let reg = Registry::new(AxiomPolicy::NoAxioms);
        let name = interner.intern("nowhere");
        let err = dependency_closure(&arena, &reg, name).unwrap_err();
        assert_eq!(err, ProofError::UndeclaredRoot(name));
    }

    #[test]
    fn checked_theorem_carries_its_own_identity_and_closure() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let prop = arena.sort(Sort::Prop);
        let mut fuel = DEFAULT_FUEL;

        // A trivial "theorem": Eq(Type 0, Prop, Prop) -- valid because
        // Prop : Type 0, so both operands genuinely have type ty0.
        // Proved by... well, we need an actual proof term. Since no
        // equality introduction rule (reflexivity) exists yet in this
        // minimal kernel (that lands with vf-axioms's built-in
        // constants or inductive types), stand in with an axiom
        // asserting it, and define the "theorem" as a direct alias of
        // that axiom -- the point of this test is CheckedTheorem's
        // bookkeeping, not proving anything non-trivial.
        let statement = arena.eq(ty0, prop, prop);
        let refl_axiom = interner.intern("refl_of_ty0");
        reg.declare_axiom(
            &mut arena,
            &mut interner,
            refl_axiom,
            statement,
            "stand-in for reflexivity",
            &mut fuel,
        )
        .unwrap();

        let proof = arena.const_(refl_axiom);
        let thm_name = interner.intern("my_theorem");
        reg.declare_theorem(
            &mut arena,
            &mut interner,
            thm_name,
            statement,
            proof,
            &mut fuel,
        )
        .unwrap();

        let thm = checked_theorem(&arena, &reg, thm_name).unwrap();
        assert_eq!(thm.name, thm_name);
        assert_eq!(thm.statement, statement);
        assert_eq!(thm.proof, proof);
        assert!(!thm.is_axiom_free(), "depends on refl_axiom");
        assert!(thm.dependencies.axioms.contains(&refl_axiom));
    }

    #[test]
    fn asking_for_a_checked_theorem_on_an_axiom_is_rejected() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("Base");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_axiom(&mut arena, &mut interner, name, ty0, "base", &mut fuel)
            .unwrap();

        let err = checked_theorem(&arena, &reg, name).unwrap_err();
        assert_eq!(err, ProofError::NotATheorem(name));
    }
}
