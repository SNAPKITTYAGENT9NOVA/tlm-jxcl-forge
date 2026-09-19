//! Bitfield extraction/insertion and sign-extension helpers used by encoding, decoding, and the ALU.
//!
//! Owns: extract_bits/insert_bits/sign_extend and related bit-level primitives.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `foundation` category. Planned public API: extract_bits, insert_bits, sign_extend.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, property.
    }
}
