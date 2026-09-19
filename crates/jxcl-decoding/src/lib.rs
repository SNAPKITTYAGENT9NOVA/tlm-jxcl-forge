//! Decodes an Instruction from its binary wire format -- the exact inverse of jxcl-encoding.
//!
//! Owns: decode(bytes) -> Instruction; the only place instruction bit layout is read.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: decode.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, property, golden, fuzz.
    }
}
