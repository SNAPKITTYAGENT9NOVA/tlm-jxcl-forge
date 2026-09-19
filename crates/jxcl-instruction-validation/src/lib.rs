//! Per-instruction operand-legality checks (register range, immediate range, addressing-mode compatibility with the opcode) shared by the decoder and the static binary validator.
//!
//! Owns: validate_instruction(&Instruction) -> Result<(), Error> -- the single source of truth for 'is this a legal instruction', so the decoder and jxcl-validator never diverge.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: validate_instruction.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, boundary, error-path.
    }
}
