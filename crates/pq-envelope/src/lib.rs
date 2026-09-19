//! The sealed-value wire format: key version, KEM ciphertext, nonce, AEAD ciphertext.
//!
//! Owns: Envelope and its to_bytes/from_bytes wire format -- the only place the envelope byte layout is defined.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: Envelope, seal, open.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, boundary, known-answer.
    }
}
