//! The mechanical spec-vs-code conformance suite: checks docs/ISA_SPEC.md and docs/RTL_CONTRACT.md's stated facts against jxcl-isa-schema and jxcl-hardware's generated RTL.
//!
//! Owns: The final mechanical verification layer tying software ISA and hardware RTL to their documentation.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `security` category. Planned public API: (test-only crate).
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // conformance.
    }
}
