//! Arithmetic/logic unit semantics: add/sub/mul/div/shift/bitwise, with flag updates.
//!
//! Owns: All arithmetic semantics -- no other crate performs ALU-equivalent computation independently.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `execution` category. Planned public API: Alu, Alu::execute.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, property, boundary.
    }
}
