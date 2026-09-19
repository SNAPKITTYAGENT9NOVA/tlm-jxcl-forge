//! The RTL description of jxcl's core datapath (opcode decoder, ALU, register file) generated as a jxcl-hdl AST directly from jxcl-opcodes/jxcl-constants/jxcl-alu -- the mechanically-verifiable software/hardware bridge required by docs/RTL_CONTRACT.md.
//!
//! Owns: build_decoder_module/build_alu_module/build_register_file_module.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `hardware` category. Planned public API: build_decoder_module, build_alu_module, build_register_file_module.
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
