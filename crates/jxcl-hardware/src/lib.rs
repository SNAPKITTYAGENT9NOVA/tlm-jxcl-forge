// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Top-level facade integrating `jxcl-rtl`'s generated core-datapath
//! modules with both text backends (`jxcl-verilog`, `jxcl-vhdl`) and the
//! toy structural synthesis pass (`jxcl-synthesis`/`jxcl-netlist`)
//! behind one `emit_verilog`/`emit_vhdl`/`emit_netlist` API.
//!
//! # Scope and honesty boundary
//!
//! Composing real code does not make the result more verified than its
//! parts: nothing in this crate simulates the emitted Verilog/VHDL, and
//! nothing feeds it to a real logic synthesizer. There is no
//! `iverilog`/`verilator`/`yosys` in this environment (see
//! `docs/HARDWARE_LIMITATIONS.md` at the workspace root). See each
//! function's docs below for exactly what it does and does not cover --
//! in particular, [`emit_netlist`] is explicit about which of the three
//! core datapath modules the (intentionally narrow) toy synthesis pass
//! can and cannot handle today.
#![forbid(unsafe_code)]

use jxcl_netlist::Netlist;
use jxcl_synthesis::SynthesisError;

/// The three `jxcl-hdl` modules that make up jxcl's core datapath, as
/// generated directly from `jxcl-rtl` (which in turn generates them
/// from the real `jxcl-opcodes`/`jxcl-constants` tables -- see
/// `jxcl-rtl`'s own docs for the `jxcl-alu` placeholder caveat on
/// [`CoreDatapath::alu`]).
pub struct CoreDatapath {
    pub decoder: jxcl_hdl::Module,
    pub alu: jxcl_hdl::Module,
    pub register_file: jxcl_hdl::Module,
}

impl CoreDatapath {
    /// Generate a fresh copy of all three core datapath modules from
    /// `jxcl-rtl`.
    pub fn generate() -> Self {
        CoreDatapath {
            decoder: jxcl_rtl::build_decoder_module(),
            alu: jxcl_rtl::build_alu_module(),
            register_file: jxcl_rtl::build_register_file_module(),
        }
    }

    fn modules(&self) -> [(&'static str, &jxcl_hdl::Module); 3] {
        [
            ("decoder", &self.decoder),
            ("alu", &self.alu),
            ("register_file", &self.register_file),
        ]
    }
}

impl Default for CoreDatapath {
    fn default() -> Self {
        Self::generate()
    }
}

/// Render all three core datapath modules to Verilog-2001 text (one
/// module per source module, concatenated in decoder/alu/register_file
/// order, separated by a blank line). See `jxcl-verilog`'s own docs for
/// exactly what "renders to Verilog" does and doesn't guarantee (text
/// that follows Verilog-2001 grammar for this AST's constructs; never
/// run through a real toolchain).
pub fn emit_verilog() -> String {
    let datapath = CoreDatapath::generate();
    datapath
        .modules()
        .iter()
        .map(|(_, m)| jxcl_verilog::render_verilog(m))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render all three core datapath modules to VHDL text, in the same
/// order and with the same honesty boundary as [`emit_verilog`] (see
/// `jxcl-vhdl`'s own docs).
pub fn emit_vhdl() -> String {
    let datapath = CoreDatapath::generate();
    datapath
        .modules()
        .iter()
        .map(|(_, m)| jxcl_vhdl::render_vhdl(m))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Per-module result of running `jxcl-synthesis`'s toy synthesis pass
/// over each of the three core datapath modules.
///
/// `jxcl-synthesis` is deliberately scoped to a single narrow shape (a
/// 1-bit-selector, 2-arm, identifier-only case statement -- see its own
/// docs). **None of the three real core datapath modules currently fit
/// that shape**: the decoder's case statement has one arm per real
/// opcode (dozens, not two) and its arm bodies assign literals rather
/// than bare identifiers, and the ALU/register-file modules (the latter
/// especially, and the former as a documented `jxcl-alu` placeholder --
/// see `jxcl-rtl`'s docs) currently carry no top-level case statement at
/// all. So today, every field here is legitimately `Err`, and that is
/// the honest, correct answer, not a bug: it says exactly what
/// `docs/HARDWARE_LIMITATIONS.md` says the toy synthesis pass is scoped
/// to, applied to what `jxcl-rtl` actually generates today. This struct
/// exists so that claim is a checked fact (see this crate's
/// `netlist_report_honestly_reports_out_of_scope_modules` test) rather
/// than an unverified assertion in a doc comment, and so it stays
/// correct automatically if a future, richer `jxcl-rtl` module *does*
/// fall in scope.
pub struct NetlistReport {
    pub decoder: Result<Netlist, SynthesisError>,
    pub alu: Result<Netlist, SynthesisError>,
    pub register_file: Result<Netlist, SynthesisError>,
}

/// Run `jxcl-synthesis::synthesize` over each of the three core
/// datapath modules and report the per-module result. See
/// [`NetlistReport`] for why every field is expected to be `Err` today.
pub fn emit_netlist() -> NetlistReport {
    let datapath = CoreDatapath::generate();
    NetlistReport {
        decoder: jxcl_synthesis::synthesize(&datapath.decoder),
        alu: jxcl_synthesis::synthesize(&datapath.alu),
        register_file: jxcl_synthesis::synthesize(&datapath.register_file),
    }
}

/// Synthesize an arbitrary `jxcl-hdl` module directly, for a caller that
/// wants to run the toy synthesis pass on something other than the
/// three standard core datapath modules (e.g. `jxcl-hdl`'s own `mux2`
/// canonical example, which *does* fit the pass's scope).
pub fn emit_netlist_for(module: &jxcl_hdl::Module) -> Result<Netlist, SynthesisError> {
    jxcl_synthesis::synthesize(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_verilog_contains_all_three_module_headers() {
        let text = emit_verilog();
        assert!(text.contains("module decoder("));
        assert!(text.contains("module alu("));
        assert!(text.contains("module register_file("));
    }

    #[test]
    fn emit_vhdl_contains_all_three_entity_headers() {
        let text = emit_vhdl();
        assert!(text.contains("entity decoder is"));
        assert!(text.contains("entity alu is"));
        assert!(text.contains("entity register_file is"));
    }

    #[test]
    fn emitted_verilog_is_balanced() {
        let text = emit_verilog();
        assert_eq!(
            text.matches("module ").count(),
            text.matches("endmodule").count()
        );
    }

    #[test]
    fn netlist_report_honestly_reports_out_of_scope_modules() {
        // Documents, as a checked fact rather than only a doc comment,
        // that none of the three real generated core-datapath modules
        // fall inside the toy synthesis pass's narrow scope today.
        let report = emit_netlist();
        assert!(report.decoder.is_err());
        assert!(report.alu.is_err());
        assert!(report.register_file.is_err());
    }

    #[test]
    fn emit_netlist_for_succeeds_on_a_module_that_fits_the_toy_scope() {
        // The canonical mux2 example is exactly the shape jxcl-synthesis
        // supports, proving the wiring (jxcl-hardware -> jxcl-synthesis
        // -> jxcl-netlist) works end to end for an in-scope module.
        let netlist = emit_netlist_for(&jxcl_hdl::examples::mux2_module())
            .expect("mux2 is in the toy synthesis pass's scope");
        assert_eq!(netlist.gates.len(), 1);
        netlist.validate().expect("well-formed netlist");
    }

    #[test]
    fn core_datapath_generate_matches_jxcl_rtl_directly() {
        let datapath = CoreDatapath::generate();
        assert_eq!(datapath.decoder.name, "decoder");
        assert_eq!(datapath.alu.name, "alu");
        assert_eq!(datapath.register_file.name, "register_file");
    }
}
