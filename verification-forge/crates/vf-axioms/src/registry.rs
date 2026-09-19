//! The declaration registry: the concrete, policy-enforcing
//! [`vf_kernel::KernelEnv`] every axiom, definition, and theorem in a
//! verification-forge session is ultimately declared into.
use crate::error::RegistryError;
use crate::policy::AxiomPolicy;
use std::collections::HashMap;
use vf_core::{Interner, Symbol, TermArena, TermId};
use vf_kernel::{Context, KernelEnv};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Axiom { ty: TermId, justification: String },
    Definition { ty: TermId, value: TermId },
}

/// A registry of declarations, checked against a chosen
/// [`AxiomPolicy`]. Names can only ever be *added*, never replaced or
/// removed: once `declare_axiom`/`declare_definition` accepts a name,
/// every later declaration sees it exactly as it was accepted,
/// forever. Combined with the kernel only ever being asked to check a
/// declaration against entries already in the registry (never
/// forward-looking ones), this makes the reference graph between
/// declared names a DAG by construction -- `vf-proof`'s dependency
/// closure relies on that.
#[derive(Debug, Clone)]
pub struct Registry {
    policy: AxiomPolicy,
    order: Vec<Symbol>,
    entries: HashMap<Symbol, Entry>,
}

impl Registry {
    pub fn new(policy: AxiomPolicy) -> Self {
        Registry {
            policy,
            order: Vec::new(),
            entries: HashMap::new(),
        }
    }

    pub fn policy(&self) -> AxiomPolicy {
        self.policy
    }

    pub fn is_declared(&self, name: Symbol) -> bool {
        self.entries.contains_key(&name)
    }

    pub fn is_axiom(&self, name: Symbol) -> bool {
        matches!(self.entries.get(&name), Some(Entry::Axiom { .. }))
    }

    /// The justification string a name's axiom was declared with, or
    /// `None` if `name` isn't a declared axiom.
    pub fn justification(&self, name: Symbol) -> Option<&str> {
        match self.entries.get(&name) {
            Some(Entry::Axiom { justification, .. }) => Some(justification),
            _ => None,
        }
    }

    /// Every declared name, in the (necessarily dependency-respecting)
    /// order it was declared in.
    pub fn declared_order(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.order.iter().copied()
    }

    /// Declare `name : ty` as an axiom, with `justification` recording
    /// *why* -- required to be non-empty unless the registry's policy
    /// is [`AxiomPolicy::Unrestricted`]. This is the only function in
    /// this crate that can introduce an axiom; nothing else does so as
    /// a side effect.
    pub fn declare_axiom(
        &mut self,
        arena: &mut TermArena,
        interner: &mut Interner,
        name: Symbol,
        ty: TermId,
        justification: impl Into<String>,
        fuel: &mut u64,
    ) -> Result<(), RegistryError> {
        if self.is_declared(name) {
            return Err(RegistryError::DuplicateName(name));
        }
        if matches!(self.policy, AxiomPolicy::NoAxioms) {
            return Err(RegistryError::AxiomsForbidden(name));
        }
        let justification = justification.into();
        if matches!(self.policy, AxiomPolicy::ExplicitAxiomsOnly) && justification.trim().is_empty()
        {
            return Err(RegistryError::MissingJustification(name));
        }
        vf_kernel::sort_of(arena, interner, self, &Context::new(), ty, fuel)?;
        self.entries
            .insert(name, Entry::Axiom { ty, justification });
        self.order.push(name);
        Ok(())
    }

    /// Declare `name : ty := value`. The kernel independently checks
    /// that `ty` is a well-formed type and that `value` has type `ty`
    /// -- this crate never takes an elaborator's word for either.
    pub fn declare_definition(
        &mut self,
        arena: &mut TermArena,
        interner: &mut Interner,
        name: Symbol,
        ty: TermId,
        value: TermId,
        fuel: &mut u64,
    ) -> Result<(), RegistryError> {
        if self.is_declared(name) {
            return Err(RegistryError::DuplicateName(name));
        }
        vf_kernel::sort_of(arena, interner, self, &Context::new(), ty, fuel)?;
        vf_kernel::check(arena, interner, self, &Context::new(), value, ty, fuel)?;
        self.entries.insert(name, Entry::Definition { ty, value });
        self.order.push(name);
        Ok(())
    }

    /// Declare `name : statement := proof`. A theorem is exactly a
    /// definition whose value happens to be a proof term and whose
    /// type happens to be (usually, though the kernel doesn't require
    /// it) a `Prop` -- Curry-Howard means there is no separate
    /// "theorem-checking" algorithm, only this same trusted mechanism.
    pub fn declare_theorem(
        &mut self,
        arena: &mut TermArena,
        interner: &mut Interner,
        name: Symbol,
        statement: TermId,
        proof: TermId,
        fuel: &mut u64,
    ) -> Result<(), RegistryError> {
        self.declare_definition(arena, interner, name, statement, proof, fuel)
    }
}

impl KernelEnv for Registry {
    fn type_of_const(&self, name: Symbol) -> Option<TermId> {
        match self.entries.get(&name)? {
            Entry::Axiom { ty, .. } | Entry::Definition { ty, .. } => Some(*ty),
        }
    }

    fn unfold(&self, name: Symbol) -> Option<TermId> {
        match self.entries.get(&name)? {
            Entry::Definition { value, .. } => Some(*value),
            Entry::Axiom { .. } => None,
        }
    }
}

/// Inherent aliases for the two [`KernelEnv`] accessors, under names
/// that read well outside a reduction context and don't require a
/// caller (e.g. `vf-proof`, which has no reason to depend on
/// `vf-kernel` just to walk dependency closures) to import the trait.
impl Registry {
    pub fn type_of(&self, name: Symbol) -> Option<TermId> {
        self.type_of_const(name)
    }

    pub fn value_of(&self, name: Symbol) -> Option<TermId> {
        self.unfold(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_core::Sort;
    use vf_reducer::DEFAULT_FUEL;

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    #[test]
    fn declaring_a_well_typed_axiom_succeeds() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("c");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_axiom(
            &mut arena,
            &mut interner,
            name,
            ty0,
            "a test axiom",
            &mut fuel,
        )
        .unwrap();
        assert!(reg.is_declared(name));
        assert!(reg.is_axiom(name));
        assert_eq!(reg.justification(name), Some("a test axiom"));
    }

    #[test]
    fn no_axioms_policy_rejects_every_axiom() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::NoAxioms);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("c");
        let mut fuel = DEFAULT_FUEL;
        let err = reg
            .declare_axiom(
                &mut arena,
                &mut interner,
                name,
                ty0,
                "irrelevant",
                &mut fuel,
            )
            .unwrap_err();
        assert_eq!(err, RegistryError::AxiomsForbidden(name));
        assert!(!reg.is_declared(name));
    }

    #[test]
    fn explicit_axioms_only_rejects_an_empty_justification() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("c");
        let mut fuel = DEFAULT_FUEL;
        let err = reg
            .declare_axiom(&mut arena, &mut interner, name, ty0, "", &mut fuel)
            .unwrap_err();
        assert_eq!(err, RegistryError::MissingJustification(name));
    }

    #[test]
    fn unrestricted_policy_allows_an_empty_justification() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::Unrestricted);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("c");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_axiom(&mut arena, &mut interner, name, ty0, "", &mut fuel)
            .unwrap();
        assert!(reg.is_declared(name));
    }

    #[test]
    fn duplicate_names_are_rejected_even_across_axiom_and_definition() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::Unrestricted);
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let name = interner.intern("c");
        let mut fuel = DEFAULT_FUEL;
        reg.declare_axiom(&mut arena, &mut interner, name, ty0, "first", &mut fuel)
            .unwrap();
        let err = reg
            .declare_definition(&mut arena, &mut interner, name, ty1, ty0, &mut fuel)
            .unwrap_err();
        assert_eq!(err, RegistryError::DuplicateName(name));
    }

    #[test]
    fn an_axiom_with_an_ill_formed_type_is_rejected_by_the_kernel() {
        // `ty` here is a FreeVar that was never bound by anything --
        // not a well-formed type, and the registry must not just take
        // the caller's word that it is one.
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::Unrestricted);
        let bogus_ty = arena.free_var(interner.intern("nowhere_bound"));
        let name = interner.intern("c");
        let mut fuel = DEFAULT_FUEL;
        let err = reg
            .declare_axiom(
                &mut arena,
                &mut interner,
                name,
                bogus_ty,
                "doesn't matter",
                &mut fuel,
            )
            .unwrap_err();
        assert!(matches!(err, RegistryError::Kernel(_)));
        assert!(!reg.is_declared(name));
    }

    #[test]
    fn declaring_a_definition_checks_the_value_against_the_stated_type() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::NoAxioms);
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let name = interner.intern("my_type");
        let mut fuel = DEFAULT_FUEL;
        // ty0 : ty1 genuinely holds (Type 0 : Type 1).
        reg.declare_definition(&mut arena, &mut interner, name, ty1, ty0, &mut fuel)
            .unwrap();
        assert_eq!(reg.type_of_const(name), Some(ty1));
        assert_eq!(reg.value_of(name), Some(ty0));
    }

    #[test]
    fn a_definition_whose_value_does_not_match_its_stated_type_is_rejected() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::NoAxioms);
        let ty0 = arena.sort(Sort::Type(0));
        let name = interner.intern("bad");
        let mut fuel = DEFAULT_FUEL;
        // Claiming ty0 (Sort(Type 0)) has type ty0 itself is false:
        // Type 0 : Type 1, not Type 0.
        let err = reg
            .declare_definition(&mut arena, &mut interner, name, ty0, ty0, &mut fuel)
            .unwrap_err();
        assert!(matches!(err, RegistryError::Kernel(_)));
    }

    #[test]
    fn a_later_definition_can_reference_an_earlier_one() {
        let (mut arena, mut interner) = setup();
        let mut reg = Registry::new(AxiomPolicy::ExplicitAxiomsOnly);
        let ty0 = arena.sort(Sort::Type(0));
        let ty1 = arena.sort(Sort::Type(1));
        let axiom_name = interner.intern("MyType");
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

        // `alias : Type 0 := MyType` -- referencing the just-declared
        // axiom by its Const; MyType's own type is Type 0, so alias's
        // stated type must match that, not Type 1.
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
        assert_eq!(reg.value_of(alias_name), Some(const_ref));

        // A forward reference (to a name not yet declared) must fail
        // -- not because of any special check, but because the kernel
        // sees an UnknownConstant.
        let forward_name = interner.intern("uses_future");
        let not_yet = interner.intern("not_yet_declared");
        let forward_ref = arena.const_(not_yet);
        let err = reg
            .declare_definition(
                &mut arena,
                &mut interner,
                forward_name,
                ty1,
                forward_ref,
                &mut fuel,
            )
            .unwrap_err();
        assert!(matches!(err, RegistryError::Kernel(_)));
    }
}
