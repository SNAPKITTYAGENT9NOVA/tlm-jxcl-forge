//! The operand/addressing-mode model (register-direct, immediate, memory-indirect, etc.).
//!
//! Owns: The Operand enum and addressing-mode variants.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: Operand, AddressingMode.
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
