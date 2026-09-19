//! HKDF-SHA256 expansion of a KEM shared secret into an AES-256 key, with fixed domain separation.
//!
//! Owns: derive_aes_key.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: derive_aes_key.
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
