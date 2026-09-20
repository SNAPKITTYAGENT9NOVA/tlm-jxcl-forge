// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The RTL description of jxcl's core datapath (opcode decoder, ALU,
//! register file), built as a `jxcl-hdl` [`jxcl_hdl::Module`] AST
//! **generated directly from `jxcl-opcodes`/`jxcl-constants`/
//! `jxcl-alu`'s real tables and constants** -- this is the
//! mechanically-verifiable software/hardware bridge required by
//! `docs/RTL_CONTRACT.md`.
//!
//! # Scope and honesty boundary
//!
//! This crate builds a hardware-description **AST**. It does not
//! simulate anything, and nothing here is checked against a real
//! Verilog/VHDL toolchain -- there is no `iverilog`/`verilator`/`yosys`
//! in this environment (see `docs/HARDWARE_LIMITATIONS.md` at the
//! workspace root). What *is* checked, mechanically, in this crate's
//! own tests, is that the generated decoder module cannot silently
//! drift from `jxcl-opcodes`' real opcode table: see
//! [`build_decoder_module`] and its
//! `decoder_case_arms_match_opcode_table_exactly` test.
//!
//! ## `jxcl-alu` status at the time this crate was written
//!
//! `jxcl-alu` is still a placeholder crate (no `Alu` type, no
//! `Alu::execute`, no per-operation table to iterate) as of this
//! writing -- it is being built concurrently by a sibling batch. Per
//! `docs/CRATE_GENERATION_PLAN.md`'s guidance for this situation,
//! [`build_alu_module`] is therefore a **documented minimal
//! placeholder** module: it only captures the width contract (two
//! `WORD_BITS`-wide operands in, one `WORD_BITS`-wide result out, an
//! operation-selector input), not a real per-operation case statement
//! generated from a real ALU operation table. Once `jxcl-alu` lands
//! with a real operation enum/table, `build_alu_module` should be
//! rewritten to generate one case arm per real ALU operation, exactly
//! mirroring how [`build_decoder_module`] is generated from
//! `jxcl-opcodes::all_defs()` today.
#![forbid(unsafe_code)]

use jxcl_constants::{OPCODE_BITS, REGISTER_COUNT, WORD_BITS};
use jxcl_hdl::{CaseValue, Expr, Module, Port, Sensitivity, Statement};
use jxcl_opcodes::all_defs;

/// The number of bits needed to address `count` distinct items
/// (`ceil(log2(count))`, with `addr_bits_for(1) == 0` and
/// `addr_bits_for(0)` treated as `0`). Used to size the register file's
/// address ports from `jxcl_constants::REGISTER_COUNT` rather than
/// hard-coding a width that could silently drift from it.
fn addr_bits_for(count: usize) -> u32 {
    if count <= 1 {
        return 0;
    }
    let mut bits = 0u32;
    while (1usize << bits) < count {
        bits += 1;
    }
    bits
}

/// The number of bits needed to hold `value` as an unsigned binary
/// literal (`value.max(1)`'s bit length), used to size the decoder's
/// `instr_len` output from the real per-format encoded lengths in
/// `jxcl-opcodes` rather than a hand-picked constant.
fn bits_for_value(value: u64) -> u32 {
    (u64::BITS - value.max(1).leading_zeros()).max(1)
}

/// Build the opcode-decoder module: one `case` arm **per real entry**
/// in `jxcl_opcodes::all_defs()`, keyed by that entry's real numeric
/// opcode. This is the crate's central mechanically-verifiable claim
/// (`docs/RTL_CONTRACT.md`): the generated decoder cannot have more,
/// fewer, or differently-numbered arms than the actual ISA opcode
/// table without the `decoder_case_arms_match_opcode_table_exactly`
/// test in this module failing.
///
/// Interface:
/// - `opcode` (input, [`jxcl_constants::OPCODE_BITS`] wide): the
///   fetched opcode byte.
/// - `valid` (output, 1 bit): high iff `opcode` matches a real
///   registry entry.
/// - `instr_len` (output, wide enough for
///   [`jxcl_constants::MAX_INSTRUCTION_LEN`]): the real encoded
///   instruction length (in bytes) for that opcode's
///   `jxcl_opcodes::Format`, taken directly from
///   `jxcl_opcodes::Format::len`.
///
/// Every matched arm sets both outputs from the real registry entry;
/// the `default` arm (no known opcode) sets `valid` low and
/// `instr_len` to 0.
pub fn build_decoder_module() -> Module {
    let defs = all_defs();
    let len_width = bits_for_value(jxcl_constants::MAX_INSTRUCTION_LEN as u64);

    let mut arms: Vec<(CaseValue, Vec<Statement>)> = defs
        .iter()
        .map(|def| {
            let value = CaseValue::literal(OPCODE_BITS, def.opcode as u64);
            let body = vec![
                Statement::assign("valid", Expr::literal(1, 1)),
                Statement::assign(
                    "instr_len",
                    Expr::literal(len_width, def.format.len() as u64),
                ),
            ];
            (value, body)
        })
        .collect();
    arms.push((
        CaseValue::Default,
        vec![
            Statement::assign("valid", Expr::literal(1, 0)),
            Statement::assign("instr_len", Expr::literal(len_width, 0)),
        ],
    ));

    Module::new("decoder")
        .with_ports(vec![
            Port::input("opcode", OPCODE_BITS),
            Port::output("valid", 1),
            Port::output("instr_len", len_width),
        ])
        .with_statements(vec![Statement::always(
            Sensitivity::Combinational,
            vec![Statement::case_stmt(Expr::ident("opcode"), arms)],
        )])
}

/// Build the register-file module's **interface**, sized directly from
/// `jxcl_constants::REGISTER_COUNT` (address port width) and
/// `jxcl_constants::WORD_BITS` (data port width) -- a two-read/one-write
/// port register file, the shape `docs/RTL_CONTRACT.md` maps to
/// `isa::registers::RegisterFile`.
///
/// This module intentionally has **no internal statements**: `jxcl-hdl`'s
/// AST (see its own crate docs) has no indexed-array/memory-cell
/// construct, so a behavioral read/write description of an actual
/// `REGISTER_COUNT`-entry storage array is out of scope for this AST as
/// it exists today. What *is* generated, and mechanically checked, is
/// the interface width contract: address ports wide enough to name
/// every register, data ports exactly `WORD_BITS` wide.
pub fn build_register_file_module() -> Module {
    let addr_width = addr_bits_for(REGISTER_COUNT);
    Module::new("register_file").with_ports(vec![
        Port::input("read_addr1", addr_width),
        Port::input("read_addr2", addr_width),
        Port::output("read_data1", WORD_BITS),
        Port::output("read_data2", WORD_BITS),
        Port::input("write_addr", addr_width),
        Port::input("write_data", WORD_BITS),
        Port::input("write_enable", 1),
    ])
}

/// Build the ALU module -- see the crate-level "`jxcl-alu` status"
/// section. `jxcl-alu` has no real `Alu`/operation table yet, so this
/// is a **documented minimal placeholder** capturing only the width
/// contract (two `WORD_BITS` operands in, one `WORD_BITS` result out,
/// an [`jxcl_constants::OPCODE_BITS`]-wide operation-selector input
/// standing in for a real operation enum), not a real per-operation
/// case statement. It carries no internal statements for the same
/// reason: there is no real per-operation semantics to lower yet.
pub fn build_alu_module() -> Module {
    Module::new("alu").with_ports(vec![
        Port::input("op_a", WORD_BITS),
        Port::input("op_b", WORD_BITS),
        // Placeholder operation selector width: reuses OPCODE_BITS as a
        // stand-in upper bound until jxcl-alu exposes a real operation
        // enum/table this crate can iterate the way build_decoder_module
        // iterates jxcl-opcodes.
        Port::input("alu_op", OPCODE_BITS),
        Port::output("result", WORD_BITS),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_hdl::Direction;
    use std::collections::BTreeSet;

    fn decoder_arms(module: &Module) -> &Vec<(CaseValue, Vec<Statement>)> {
        match module
            .top_level_case()
            .expect("decoder module must have a top-level case statement")
        {
            Statement::Case { arms, .. } => arms,
            _ => unreachable!(),
        }
    }

    /// THE mechanical proof required by `docs/RTL_CONTRACT.md` /
    /// rule #14: the generated decoder's case statement has exactly as
    /// many non-default arms as `jxcl-opcodes` has real table entries,
    /// and every arm's literal value is a real opcode id -- no more, no
    /// fewer, none invented, none dropped.
    #[test]
    fn decoder_case_arms_match_opcode_table_exactly() {
        let module = build_decoder_module();
        let arms = decoder_arms(&module);

        let generated_ids: BTreeSet<u64> = arms
            .iter()
            .filter_map(|(value, _)| match value {
                CaseValue::Literal { value, .. } => Some(*value),
                CaseValue::Default => None,
            })
            .collect();
        let real_ids: BTreeSet<u64> = all_defs().iter().map(|d| d.opcode as u64).collect();

        assert_eq!(
            generated_ids, real_ids,
            "generated decoder arm opcode ids must exactly match jxcl-opcodes::all_defs()"
        );
        assert_eq!(
            generated_ids.len(),
            all_defs().len(),
            "no two real opcodes may have collapsed into one generated arm"
        );

        // Exactly one default arm, and no other duplicate arm values.
        let default_count = arms
            .iter()
            .filter(|(v, _)| matches!(v, CaseValue::Default))
            .count();
        assert_eq!(default_count, 1);
        assert_eq!(arms.len(), all_defs().len() + 1);
    }

    #[test]
    fn decoder_opcode_port_width_matches_constants() {
        let module = build_decoder_module();
        let opcode_port = module
            .ports
            .iter()
            .find(|p| p.name == "opcode")
            .expect("decoder has an opcode port");
        assert_eq!(opcode_port.direction, Direction::In);
        assert_eq!(opcode_port.width, OPCODE_BITS);
    }

    #[test]
    fn decoder_arm_instr_len_matches_real_format_length() {
        let module = build_decoder_module();
        let arms = decoder_arms(&module);
        for def in all_defs() {
            let (_, body) = arms
                .iter()
                .find(|(v, _)| matches!(v, CaseValue::Literal { value, .. } if *value == def.opcode as u64))
                .expect("every real opcode has a generated arm");
            let len_assign = body
                .iter()
                .find_map(|s| match s {
                    Statement::Assign { target, expr } if target == "instr_len" => Some(expr),
                    _ => None,
                })
                .expect("arm assigns instr_len");
            match len_assign {
                Expr::Literal { value, .. } => {
                    assert_eq!(*value, def.format.len() as u64);
                }
                other => panic!("expected a literal instr_len assignment, got {other:?}"),
            }
        }
    }

    #[test]
    fn register_file_ports_match_constants() {
        let module = build_register_file_module();
        let expected_addr_width = addr_bits_for(REGISTER_COUNT);
        // 32 registers need exactly 5 address bits.
        assert_eq!(expected_addr_width, 5);

        let read_addr1 = module
            .ports
            .iter()
            .find(|p| p.name == "read_addr1")
            .unwrap();
        assert_eq!(read_addr1.width, expected_addr_width);
        assert_eq!(read_addr1.direction, Direction::In);

        let write_data = module
            .ports
            .iter()
            .find(|p| p.name == "write_data")
            .unwrap();
        assert_eq!(write_data.width, WORD_BITS);
        assert_eq!(write_data.direction, Direction::In);

        let read_data1 = module
            .ports
            .iter()
            .find(|p| p.name == "read_data1")
            .unwrap();
        assert_eq!(read_data1.width, WORD_BITS);
        assert_eq!(read_data1.direction, Direction::Out);
    }

    #[test]
    fn alu_module_ports_carry_word_width_operands() {
        let module = build_alu_module();
        assert_eq!(module.inputs().count(), 3);
        assert_eq!(module.outputs().count(), 1);
        for name in ["op_a", "op_b"] {
            let p = module.ports.iter().find(|p| p.name == name).unwrap();
            assert_eq!(p.width, WORD_BITS);
        }
        let result = module.ports.iter().find(|p| p.name == "result").unwrap();
        assert_eq!(result.width, WORD_BITS);
        assert_eq!(result.direction, Direction::Out);
    }

    #[test]
    fn addr_bits_for_matches_expected_powers_of_two() {
        assert_eq!(addr_bits_for(1), 0);
        assert_eq!(addr_bits_for(2), 1);
        assert_eq!(addr_bits_for(3), 2);
        assert_eq!(addr_bits_for(32), 5);
        assert_eq!(addr_bits_for(33), 6);
    }

    #[test]
    fn bits_for_value_matches_expected_widths() {
        assert_eq!(bits_for_value(0), 1);
        assert_eq!(bits_for_value(1), 1);
        assert_eq!(bits_for_value(10), 4);
        assert_eq!(bits_for_value(255), 8);
    }
}
