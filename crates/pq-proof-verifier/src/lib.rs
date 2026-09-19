//! A verifier facade that looks up the right scheme via pq-proof-registry and verifies, so callers never touch arkworks types directly.
//!
//! Owns: verify_attestation(scheme_id, ...).
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `proof` category. Planned public API: verify_attestation.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, integration.
    }
}
