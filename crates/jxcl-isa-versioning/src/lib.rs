//! ISA/binary-format version negotiation so old binaries fail closed against an incompatible newer decoder rather than silently misdecoding.
//!
//! Owns: IsaVersion, its embedding in the binary container header, and the compatibility check.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: IsaVersion, IsaVersion::is_compatible_with.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, boundary.
    }
}
