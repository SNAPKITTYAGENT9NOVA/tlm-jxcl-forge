//! Backend-independent proof types: the ProofScheme trait and shared Attestation/Error types, so callers never depend on a concrete proof backend directly.
//!
//! Owns: The ProofScheme trait.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `proof` category. Planned public API: ProofScheme, AttestationBytes, ProofError.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit.
    }
}
