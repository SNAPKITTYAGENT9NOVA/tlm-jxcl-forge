//! A registry mapping scheme-id strings to boxed ProofScheme implementations, supporting future proof schemes beyond Groth16-Pedersen.
//!
//! Owns: ProofRegistry::register/get.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `proof` category. Planned public API: ProofRegistry.
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
