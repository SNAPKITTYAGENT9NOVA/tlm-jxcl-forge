//! Architecture-wide constants: word width, register count, memory size, opcode width.
//!
//! Owns: The single authoritative set of ISA width/count constants.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `foundation` category. Planned public API: WORD_BITS, REGISTER_COUNT, MEMORY_SIZE, OPCODE_BITS.
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
