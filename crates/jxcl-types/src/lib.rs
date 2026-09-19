//! Shared newtype wrappers for architectural primitive values (word, address, register index, immediate) so every ISA/execution crate agrees on one representation.
//!
//! Owns: The Word/Address/RegisterIndex/Immediate newtypes and their arithmetic/conversion impls.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `foundation` category. Planned public API: Word, Address, RegisterIndex, Immediate.
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
