//! Top-level facade integrating the HDL AST, both text backends, and the toy synthesis pass behind one emit API.
//!
//! Owns: emit_verilog/emit_vhdl/emit_netlist entry points for the jxcl core datapath.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `hardware` category. Planned public API: emit_verilog, emit_vhdl, emit_netlist.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // integration.
    }
}
