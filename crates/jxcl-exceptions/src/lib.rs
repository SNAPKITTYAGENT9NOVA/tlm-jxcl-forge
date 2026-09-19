//! The machine's exception model: illegal opcode, misaligned access, division by zero, and the trap-handling hook.
//!
//! Owns: The Exception enum and the trap dispatch contract.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `execution` category. Planned public API: Exception, TrapHandler.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, error-path.
    }
}
