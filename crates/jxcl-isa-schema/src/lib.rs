//! A serde-serializable schema of the ISA generated from jxcl-opcodes/jxcl-constants/jxcl-registers/jxcl-flags, plus a mechanical cross-check against the numbers documented in docs/ISA_SPEC.md.
//!
//! Owns: IsaSchema and the spec-vs-code conformance check that ISA_SPEC.md's stated widths/opcode-count match the generated schema.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `isa` category. Planned public API: IsaSchema, IsaSchema::generate, IsaSchema::check_against_spec.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, conformance.
    }
}
