// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Golden-file regression tests for `jxcl-hardware`'s emitted
//! Verilog/VHDL/netlist, plus the mechanical width/opcode-count
//! cross-check against the ISA source of truth.
//!
//! # Scope and honesty boundary
//!
//! This is a test-only crate: it has no public API of its own beyond
//! [`render_netlist_text`], a small deterministic text serialization of
//! a [`jxcl_netlist::Netlist`] used purely so a netlist can be compared
//! against a checked-in golden file the same way rendered Verilog/VHDL
//! text is. Nothing in this crate simulates the generated RTL or feeds
//! it to a real Verilog/VHDL/synthesis toolchain -- there is no
//! `iverilog`/`verilator`/`yosys` in this environment (see
//! `docs/HARDWARE_LIMITATIONS.md` at the workspace root). "Passes" here
//! means: matches a checked-in golden file, or matches a structural fact
//! mechanically computed from `jxcl-opcodes`/`jxcl-constants`'s real
//! tables/constants -- not "was hardware-verified."
//!
//! ## `jxcl-isa-schema` status at the time this crate was written
//!
//! `jxcl-isa-schema` (this crate's other declared dependency, per
//! `docs/crates.toml`) is still a placeholder -- no `IsaSchema` type, no
//! `IsaSchema::generate`/`check_against_spec` -- as of this writing. The
//! `tests/isa_conformance.rs` cross-check in this crate therefore
//! compares `jxcl-hardware`'s generated decoder directly against
//! `jxcl-opcodes`/`jxcl-constants`'s real tables/constants, exactly the
//! way `jxcl-rtl`'s own conformance test does. Once `jxcl-isa-schema`
//! lands with a real generated schema, that test should be switched to
//! compare against `IsaSchema::generate()` instead (so this crate is
//! checking the RTL against the ISA's own schema layer, one level higher
//! than `jxcl-rtl`'s direct-table check), without changing what it
//! actually asserts.
#![forbid(unsafe_code)]

use jxcl_netlist::{GateKind, Netlist};

/// A small, deterministic text serialization of a [`Netlist`]: one line
/// per gate, sorted by output name so the result doesn't depend on
/// construction order, in the form `kind(in1, in2, ...) -> output`.
///
/// This exists only so a [`Netlist`] (which has no inherent canonical
/// text form of its own -- it's a structural Rust value, not a hardware
/// description) can be golden-file tested the same way rendered
/// Verilog/VHDL text already is. It is not a hardware description
/// format and nothing reads it back.
pub fn render_netlist_text(netlist: &Netlist) -> String {
    let mut lines: Vec<String> = netlist
        .gates
        .iter()
        .map(|g| {
            let kind = match g.kind {
                GateKind::And => "AND",
                GateKind::Or => "OR",
                GateKind::Not => "NOT",
                GateKind::Mux => "MUX",
            };
            format!("{kind}({}) -> {}", g.inputs.join(", "), g.output)
        })
        .collect();
    lines.sort();

    let mut wires: Vec<String> = netlist
        .wires
        .iter()
        .map(|w| format!("wire {} [{} bits]", w.name, w.width))
        .collect();
    wires.sort();

    let mut out = lines.join("\n");
    if !wires.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&wires.join("\n"));
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_netlist::{Gate, Wire};

    #[test]
    fn render_netlist_text_is_sorted_and_deterministic() {
        let net = Netlist::new()
            .with_gates(vec![
                Gate::mux("sel", "a", "b", "z"),
                Gate::not("x", "a_inv"),
            ])
            .with_wires(vec![Wire::new("internal", 4)]);
        let text = render_netlist_text(&net);
        let expected = "MUX(sel, a, b) -> z\nNOT(x) -> a_inv\nwire internal [4 bits]\n";
        assert_eq!(text, expected);
    }

    #[test]
    fn render_netlist_text_handles_empty_netlist() {
        let net = Netlist::new();
        assert_eq!(render_netlist_text(&net), "\n");
    }
}
