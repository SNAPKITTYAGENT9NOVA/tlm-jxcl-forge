//! ML-DSA (NIST FIPS 204 / Dilithium) signing and verification -- a second, complementary post-quantum primitive (authenticity, alongside pq-kem's confidentiality).
//!
//! Owns: SigningKey/VerifyingKey and sign/verify.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: SigningKey, VerifyingKey, sign, verify.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, known-answer, tamper, wrong-key.
    }
}
