//! AES-256-GCM authenticated encryption/decryption of the plaintext under the derived key.
//!
//! Owns: aead_encrypt/aead_decrypt.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: aead_encrypt, aead_decrypt.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, tamper, wrong-key.
    }
}
