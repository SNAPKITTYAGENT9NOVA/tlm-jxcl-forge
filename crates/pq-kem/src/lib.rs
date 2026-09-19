//! ML-KEM-768 (NIST FIPS 203) key generation and encapsulation/decapsulation.
//!
//! Owns: KeyPair and the raw KEM step.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: KeyPair, EncapsulationKey, DecapsulationKey.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, known-answer.
    }
}
