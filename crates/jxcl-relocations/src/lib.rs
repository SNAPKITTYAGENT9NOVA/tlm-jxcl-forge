//! Relocation entry types and the patch-in-place logic that fixes up an encoded instruction once a symbol's final address is known.
//!
//! Owns: Relocation and apply_relocation.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `toolchain` category. Planned public API: Relocation, apply_relocation.
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
