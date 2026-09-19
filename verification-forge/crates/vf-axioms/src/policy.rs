//! What axiom introduction is allowed to look like in a given
//! [`crate::Registry`]. Every policy still requires axioms to be
//! introduced through [`crate::Registry::declare_axiom`]'s own,
//! explicit, auditable call -- there is no code path anywhere in this
//! crate that adds an axiom as a side effect of something else. What
//! the policy controls is *whether an axiom may be declared at all*,
//! and *how much justification declaring one requires*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxiomPolicy {
    /// No axioms may ever be registered. Every declaration must be a
    /// [`crate::Registry::declare_definition`] whose value the kernel
    /// independently type-checks against its stated type -- the
    /// strictest, fully-constructive mode.
    NoAxioms,
    /// Axioms may be registered, but [`crate::Registry::declare_axiom`]
    /// rejects an empty justification string. This is the recommended
    /// default: it can't stop a bad justification, but it can and does
    /// stop a missing one.
    ExplicitAxiomsOnly,
    /// Axioms may be registered with or without a justification
    /// string. Still never *implicit*: this only relaxes the mandatory
    /// non-empty justification, not the requirement that axiom
    /// introduction always be its own explicit call.
    Unrestricted,
}
