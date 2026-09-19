//! A relocatable object-file format (sections + symbol table + relocation table) distinct from the final linked binary container.
//!
//! Owns: ObjectFile serialization/deserialization.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `toolchain` category. Planned public API: ObjectFile.
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
