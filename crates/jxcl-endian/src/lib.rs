//! The ISA's fixed-endianness integer <-> byte conversions, isolated from the general byte-buffer cursor.
//!
//! Owns: to_arch_bytes/from_arch_bytes for each integer width.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `foundation` category. Planned public API: to_arch_bytes, from_arch_bytes.
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
