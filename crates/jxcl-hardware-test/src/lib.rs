//! Golden-file regression tests for generated Verilog/VHDL/netlist, plus the mechanical width/opcode-count cross-check against the ISA schema.
//!
//! Owns: The hardware golden files and the ISA-vs-RTL conformance check.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `hardware` category. Planned public API: (test-only crate).
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // golden, conformance.
    }
}
