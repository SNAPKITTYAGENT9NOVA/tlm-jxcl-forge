//! The flags register: zero/carry/overflow/negative/interrupt-mask bit semantics.
//!
//! Owns: The Flags type and its bit-level get/set semantics.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: Flags.
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
