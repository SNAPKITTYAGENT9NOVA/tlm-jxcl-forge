//! The single authoritative opcode registry: ids, mnemonics, operand-shape metadata.
//!
//! Owns: The opcode table -- no other crate may define or duplicate opcode identifiers.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: Opcode, OPCODE_TABLE, opcode_by_mnemonic.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, golden.
    }
}
