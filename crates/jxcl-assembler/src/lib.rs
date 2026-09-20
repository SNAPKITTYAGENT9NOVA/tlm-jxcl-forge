// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The two-pass assembler: source text to an object file (plus a convenience path assembling and linking a single file straight to a binary, preserving the original single-step CLI workflow).
//!
//! # Public API
//!
//! - [`assemble`]: Assemble source text into a relocatable object file.
//! - [`assemble_to_binary`]: Assemble source text and link into a final binary (convenience for single-file workflows).

#![forbid(unsafe_code)]

use std::collections::HashMap;

use jxcl_binary::BinaryContainer;
use jxcl_encoding::encode;
use jxcl_errors::{AssemblerError, AssemblerErrorKind};
use jxcl_instructions::{DecodedInstruction, Operands};
use jxcl_linker::link;
use jxcl_object::{ObjectFile, RelocationTarget, TargetedRelocation};
use jxcl_opcodes::{lookup_mnemonic, Mnemonic};
use jxcl_parser::{AstNode, DataWidth, InstrNode, OperandNode, ValueExpr};
use jxcl_registers::Register;
use jxcl_relocations::{Relocation, RelocationKind};
use jxcl_symbols::SymbolSection;

fn err(kind: AssemblerErrorKind, line: usize, reason: impl Into<String>) -> AssemblerError {
    AssemblerError {
        kind,
        line,
        reason: reason.into(),
    }
}

fn data_width_bytes(w: DataWidth) -> u64 {
    match w {
        DataWidth::Byte => 1,
        DataWidth::Word => 2,
        DataWidth::Dword => 4,
        DataWidth::Qword => 8,
    }
}

fn fits_width(val: i128, bits: u32) -> bool {
    let min: i128 = -(1i128 << (bits - 1));
    let max: i128 = (1i128 << bits) - 1;
    val >= min && val <= max
}

/// A parsed-but-not-yet-resolved program: one entry per source line that
/// produced a statement, plus its (optional) label and 1-based line number.
struct ParsedLine {
    line: usize,
    label: Option<String>,
    stmt: AstNode,
}

fn parse_source(source: &str) -> Result<Vec<ParsedLine>, AssemblerError> {
    let parsed_lines = jxcl_parser::parse(source)?;
    let mut out = Vec::new();
    for pl in parsed_lines {
        out.push(ParsedLine {
            line: pl.line,
            label: pl.label,
            stmt: pl.node,
        });
    }
    Ok(out)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Code,
    Data,
}

/// Assemble JXCL assembly source into a relocatable object file.
pub fn assemble(source: &str) -> Result<ObjectFile, AssemblerError> {
    let lines = parse_source(source)?;

    // ---- Pass 1: layout ----
    let mut section = Section::Code;
    let mut code_len: u64 = 0;
    let mut data_len: u64 = 0;
    let mut symbols: HashMap<String, (Section, u64)> = HashMap::new();
    let mut consts: HashMap<String, i128> = HashMap::new();
    let mut entry_symbol: Option<String> = None;

    for pl in &lines {
        if let Some(name) = &pl.label {
            if symbols.contains_key(name) {
                return Err(err(
                    AssemblerErrorKind::DuplicateLabel,
                    pl.line,
                    format!("label {:?} redefined", name),
                ));
            }
            let sym = match section {
                Section::Code => (Section::Code, code_len),
                Section::Data => (Section::Data, data_len),
            };
            symbols.insert(name.clone(), sym);
        }
        match &pl.stmt {
            AstNode::Empty => {}
            AstNode::SwitchToCode => section = Section::Code,
            AstNode::SwitchToData => section = Section::Data,
            AstNode::ConstDef { name, value } => {
                if consts.contains_key(name) {
                    return Err(err(
                        AssemblerErrorKind::DuplicateLabel,
                        pl.line,
                        format!("constant {:?} redefined", name),
                    ));
                }
                let v = match value {
                    ValueExpr::Number(n) => *n,
                    ValueExpr::Name(other) => *consts.get(other).ok_or_else(|| {
                        err(
                            AssemblerErrorKind::UnknownLabel,
                            pl.line,
                            format!("constant {:?} referenced before definition", other),
                        )
                    })?,
                };
                consts.insert(name.clone(), v);
            }
            AstNode::EntryDef { value } => {
                if let ValueExpr::Name(name) = value {
                    entry_symbol = Some(name.clone());
                }
            }
            AstNode::Data { width, values } => {
                data_len += data_width_bytes(*width) * values.len() as u64;
            }
            AstNode::Instruction(InstrNode { mnemonic, .. }) => {
                if section != Section::Code {
                    return Err(err(
                        AssemblerErrorKind::UnexpectedToken,
                        pl.line,
                        "instructions are only allowed in the code section (.code)",
                    ));
                }
                let m = Mnemonic::from_text(&mnemonic.to_ascii_uppercase()).ok_or_else(|| {
                    err(
                        AssemblerErrorKind::UnknownMnemonic,
                        pl.line,
                        mnemonic.clone(),
                    )
                })?;
                code_len += lookup_mnemonic(m).format.len() as u64;
            }
        }
    }

    let total_code_len = code_len;

    let resolve_name = |name: &str, line: usize| -> Result<i128, AssemblerError> {
        if let Some(c) = consts.get(name) {
            return Ok(*c);
        }
        match symbols.get(name) {
            Some((Section::Code, offset)) => Ok(*offset as i128),
            Some((Section::Data, offset)) => Ok(total_code_len as i128 + *offset as i128),
            None => Err(err(
                AssemblerErrorKind::UnknownLabel,
                line,
                format!("undefined symbol {:?}", name),
            )),
        }
    };

    let eval_value = |v: &ValueExpr, line: usize| -> Result<i128, AssemblerError> {
        match v {
            ValueExpr::Number(n) => Ok(*n),
            ValueExpr::Name(name) => resolve_name(name, line),
        }
    };

    // ---- Pass 2: emission ----
    let mut code_pos: u64 = 0;
    let mut code_bytes: Vec<u8> = Vec::new();
    let mut data_bytes: Vec<u8> = Vec::new();
    let mut relocations: Vec<TargetedRelocation> = Vec::new();

    for pl in &lines {
        match &pl.stmt {
            AstNode::Empty | AstNode::ConstDef { .. } | AstNode::EntryDef { .. } => {}
            AstNode::SwitchToCode => {}
            AstNode::SwitchToData => {}
            AstNode::Data { width, values } => {
                let bits = data_width_bytes(*width) as u32 * 8;
                for v in values {
                    let val = eval_value(v, pl.line)?;
                    if !fits_width(val, bits) {
                        return Err(err(
                            AssemblerErrorKind::ImmediateOutOfRange,
                            pl.line,
                            format!("{} does not fit in {} bits", val, bits),
                        ));
                    }
                    match width {
                        DataWidth::Byte => data_bytes.push(val as u8),
                        DataWidth::Word => {
                            data_bytes.extend_from_slice(&(val as u16).to_le_bytes())
                        }
                        DataWidth::Dword => {
                            data_bytes.extend_from_slice(&(val as u32).to_le_bytes())
                        }
                        DataWidth::Qword => {
                            data_bytes.extend_from_slice(&(val as u64).to_le_bytes())
                        }
                    }
                }
            }
            AstNode::Instruction(node) => {
                let instr =
                    emit_instruction(node, code_pos, pl.line, &eval_value, &mut relocations)?;
                let start_len = code_bytes.len();
                encode(&instr, &mut code_bytes).map_err(|e| {
                    err(
                        AssemblerErrorKind::MalformedOperand,
                        pl.line,
                        format!("internal encoder rejection: {}", e),
                    )
                })?;
                code_pos += (code_bytes.len() - start_len) as u64;
            }
        }
    }

    // Build symbol table for the object file
    let mut obj = ObjectFile::new(code_bytes, data_bytes);

    // Add code section symbols (with section-local addresses)
    for (name, (section, offset)) in &symbols {
        if *section == Section::Code {
            obj.symbols
                .define(name, *offset, SymbolSection::Code)
                .map_err(|e| err(AssemblerErrorKind::UnknownLabel, 0, e.to_string()))?;
        }
    }

    // Add data section symbols (with section-local addresses)
    for (name, (section, offset)) in &symbols {
        if *section == Section::Data {
            obj.symbols
                .define(name, *offset, SymbolSection::Data)
                .map_err(|e| err(AssemblerErrorKind::UnknownLabel, 0, e.to_string()))?;
        }
    }

    // Apply relocations for forward references
    obj.relocations = relocations;

    // Set entry symbol if provided
    if let Some(name) = entry_symbol {
        obj.entry_symbol = Some(name);
    }

    Ok(obj)
}

fn emit_instruction(
    node: &InstrNode,
    code_pos: u64,
    line: usize,
    eval_value: &dyn Fn(&ValueExpr, usize) -> Result<i128, AssemblerError>,
    relocations: &mut Vec<TargetedRelocation>,
) -> Result<DecodedInstruction, AssemblerError> {
    let mnemonic = Mnemonic::from_text(&node.mnemonic.to_ascii_uppercase()).ok_or_else(|| {
        err(
            AssemblerErrorKind::UnknownMnemonic,
            line,
            node.mnemonic.clone(),
        )
    })?;
    let def = lookup_mnemonic(mnemonic);
    let pc_after_fetch = code_pos + def.format.len() as u64;

    let operands = match def.format {
        jxcl_opcodes::Format::None => {
            check_arity(node, 0, line)?;
            Operands::None
        }
        jxcl_opcodes::Format::R => {
            check_arity(node, 1, line)?;
            Operands::R {
                rd: expect_reg(&node.operands[0], line)?,
            }
        }
        jxcl_opcodes::Format::RR => {
            check_arity(node, 2, line)?;
            Operands::RR {
                rd: expect_reg(&node.operands[0], line)?,
                rs: expect_reg(&node.operands[1], line)?,
            }
        }
        jxcl_opcodes::Format::RRR => {
            check_arity(node, 3, line)?;
            Operands::RRR {
                rd: expect_reg(&node.operands[0], line)?,
                rs1: expect_reg(&node.operands[1], line)?,
                rs2: expect_reg(&node.operands[2], line)?,
            }
        }
        jxcl_opcodes::Format::RImm64 => {
            check_arity(node, 2, line)?;
            let rd = expect_reg(&node.operands[0], line)?;
            let value_expr = expect_value(&node.operands[1], line)?;

            // Check if this is a label reference (needs relocation)
            if let ValueExpr::Name(name) = &value_expr {
                // Create relocation for this immediate
                relocations.push(TargetedRelocation::new(
                    RelocationTarget::Code,
                    Relocation::new(code_pos + 1, RelocationKind::Absolute64, name.clone()),
                ));
                // Emit placeholder value (will be patched by linker)
                Operands::RImm64 { rd, imm: 0 }
            } else {
                let v = eval_value(&value_expr, line)?;
                if !fits_width(v, 64) {
                    return Err(err(
                        AssemblerErrorKind::ImmediateOutOfRange,
                        line,
                        format!("{} does not fit in 64 bits", v),
                    ));
                }
                Operands::RImm64 { rd, imm: v as u64 }
            }
        }
        jxcl_opcodes::Format::RMem => {
            check_arity(node, 2, line)?;
            let rd = expect_reg(&node.operands[0], line)?;
            let (base, disp_expr) = expect_mem(&node.operands[1], line)?;
            let disp = eval_value(&disp_expr, line)?;
            if !fits_width(disp, 32) {
                return Err(err(
                    AssemblerErrorKind::ImmediateOutOfRange,
                    line,
                    format!("displacement {} does not fit in 32 bits", disp),
                ));
            }
            Operands::RMem {
                rd,
                base,
                disp: disp as i32,
            }
        }
        jxcl_opcodes::Format::MemR => {
            check_arity(node, 2, line)?;
            let (base, disp_expr) = expect_mem(&node.operands[0], line)?;
            let disp = eval_value(&disp_expr, line)?;
            if !fits_width(disp, 32) {
                return Err(err(
                    AssemblerErrorKind::ImmediateOutOfRange,
                    line,
                    format!("displacement {} does not fit in 32 bits", disp),
                ));
            }
            let rs = expect_reg(&node.operands[1], line)?;
            Operands::MemR {
                base,
                disp: disp as i32,
                rs,
            }
        }
        jxcl_opcodes::Format::Cas => {
            check_arity(node, 3, line)?;
            let rd = expect_reg(&node.operands[0], line)?;
            let (base, disp_expr) = expect_mem(&node.operands[1], line)?;
            let disp = eval_value(&disp_expr, line)?;
            if !fits_width(disp, 32) {
                return Err(err(
                    AssemblerErrorKind::ImmediateOutOfRange,
                    line,
                    format!("displacement {} does not fit in 32 bits", disp),
                ));
            }
            let rs_new = expect_reg(&node.operands[2], line)?;
            Operands::Cas {
                rd,
                base,
                rs_new,
                disp: disp as i32,
            }
        }
        jxcl_opcodes::Format::BranchImm32 => {
            check_arity(node, 1, line)?;
            let value_expr = expect_value(&node.operands[0], line)?;

            // Check if this is a label reference (needs relocation)
            if let ValueExpr::Name(name) = &value_expr {
                // Create relocation for this branch target (PC-relative)
                relocations.push(TargetedRelocation::new(
                    RelocationTarget::Code,
                    Relocation::new(code_pos + 1, RelocationKind::PcRelative32, name.clone()),
                ));
                // Emit placeholder value (will be patched by linker)
                Operands::BranchImm32 { disp: 0 }
            } else {
                let target = eval_value(&value_expr, line)?;
                let disp = target - pc_after_fetch as i128;
                if !fits_width(disp, 32) {
                    return Err(err(
                        AssemblerErrorKind::ImmediateOutOfRange,
                        line,
                        format!(
                            "branch target {} is out of ±2GiB range from {}",
                            target, pc_after_fetch
                        ),
                    ));
                }
                Operands::BranchImm32 { disp: disp as i32 }
            }
        }
        jxcl_opcodes::Format::Imm16 => {
            check_arity(node, 1, line)?;
            let v = eval_value(&expect_value(&node.operands[0], line)?, line)?;
            if !fits_width(v, 16) {
                return Err(err(
                    AssemblerErrorKind::ImmediateOutOfRange,
                    line,
                    format!("{} does not fit in 16 bits", v),
                ));
            }
            Operands::Imm16 { imm: v as u16 }
        }
    };

    Ok(DecodedInstruction::new(mnemonic, operands))
}

fn expect_reg(op: &OperandNode, line: usize) -> Result<Register, AssemblerError> {
    match op {
        OperandNode::Reg(r) => Ok(Register::new(*r)),
        _ => Err(err(
            AssemblerErrorKind::MalformedOperand,
            line,
            "expected a register operand",
        )),
    }
}

fn expect_mem(op: &OperandNode, line: usize) -> Result<(Register, ValueExpr), AssemblerError> {
    match op {
        OperandNode::Mem { base, disp } => Ok((Register::new(*base), disp.clone())),
        _ => Err(err(
            AssemblerErrorKind::MalformedOperand,
            line,
            "expected a memory operand [base+disp]",
        )),
    }
}

fn expect_value(op: &OperandNode, line: usize) -> Result<ValueExpr, AssemblerError> {
    match op {
        OperandNode::Value(v) => Ok(v.clone()),
        _ => Err(err(
            AssemblerErrorKind::MalformedOperand,
            line,
            "expected an immediate, label, or constant",
        )),
    }
}

fn check_arity(node: &InstrNode, expected: usize, line: usize) -> Result<(), AssemblerError> {
    if node.operands.len() != expected {
        return Err(err(
            AssemblerErrorKind::OperandCountMismatch,
            line,
            format!(
                "{} expects {} operand(s), got {}",
                node.mnemonic,
                expected,
                node.operands.len()
            ),
        ));
    }
    Ok(())
}

/// Assemble source text and link into a final executable binary (convenience
/// for single-file workflows, preserving the original CLI's single-step behavior).
pub fn assemble_to_binary(source: &str) -> Result<BinaryContainer, AssemblerError> {
    let obj = assemble(source)?;
    link(&[obj]).map_err(|e| {
        err(
            AssemblerErrorKind::UnknownLabel,
            0,
            format!("linking failed: {}", e),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_simple_program_to_object() {
        let src = "\
start:
    MOVI R1, 10
    MOVI R2, 20
    ADD  R1, R2
    HALT
";
        let obj = assemble(src).unwrap();
        assert!(!obj.code.is_empty());
        assert!(obj.symbols.resolve("start").is_some());
    }

    #[test]
    fn assembles_to_binary_end_to_end() {
        let src = "\
    MOVI R1, 10
    HALT
";
        let bin = assemble_to_binary(src).unwrap();
        assert!(!bin.code.is_empty());
        assert_eq!(bin.entry_point, 0);
    }

    #[test]
    fn resolves_forward_branch_label() {
        let src = "\
    JMP skip
    HALT
skip:
    HALT
";
        let obj = assemble(src).unwrap();
        // JMP has relocation, HALT, HALT
        assert!(obj.symbols.resolve("skip").is_some());
    }

    #[test]
    fn unknown_mnemonic_is_an_error() {
        let err = assemble("BOGUS R1, R2").unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::UnknownMnemonic);
    }

    #[test]
    fn duplicate_label_is_an_error() {
        let src = "a: HALT\na: HALT\n";
        let err = assemble(src).unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::DuplicateLabel);
    }

    #[test]
    fn data_section_works() {
        let src = "\
    MOVI R1, 0
    HALT
.data
buffer:
    .qword 42
";
        let obj = assemble(src).unwrap();
        assert!(!obj.data.is_empty());
        assert_eq!(obj.data, 42u64.to_le_bytes());
        // buffer should be defined in data section
        let sym = obj.symbols.resolve("buffer").unwrap();
        assert_eq!(sym.section, SymbolSection::Data);
    }

    #[test]
    fn immediate_out_of_range_is_rejected() {
        let err = assemble(".data\n.byte 300\n").unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::ImmediateOutOfRange);
    }

    #[test]
    fn constants_work() {
        let src = "\
.const MYVAL 42
    MOVI R1, MYVAL
    HALT
";
        let obj = assemble(src).unwrap();
        assert!(!obj.code.is_empty());
    }

    #[test]
    fn constant_forward_reference_error() {
        let src = ".const A B\n.const B 10\n";
        let err = assemble(src).unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::UnknownLabel);
    }

    #[test]
    fn duplicate_constant_is_error() {
        let src = ".const A 10\n.const A 20\n";
        let err = assemble(src).unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::DuplicateLabel);
    }

    #[test]
    fn instruction_outside_code_section_error() {
        let src = ".data\nHALT\n";
        let err = assemble(src).unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::UnexpectedToken);
    }

    #[test]
    fn golden_multi_instruction_program() {
        let src = "\
start:
    MOVI R1, 5
    MOVI R2, 10
    ADD R1, R2
    HALT
";
        let obj = assemble(src).unwrap();
        // Verify we have a non-empty code section
        assert!(!obj.code.is_empty());
        // Verify start label exists
        assert!(obj.symbols.resolve("start").is_some());
        let start_sym = obj.symbols.resolve("start").unwrap();
        assert_eq!(start_sym.address, 0);
        assert_eq!(start_sym.section, SymbolSection::Code);
    }

    #[test]
    fn golden_code_and_data_sections() {
        let src = "\
main:
    MOVI R1, buffer
    LOAD R2, [R1 + 0]
    HALT

.data
buffer:
    .qword 0x123456789ABCDEF0
";
        let obj = assemble(src).unwrap();
        assert!(!obj.code.is_empty());
        assert!(!obj.data.is_empty());

        // Verify data matches expected value
        assert_eq!(obj.data, 0x123456789ABCDEF0u64.to_le_bytes());

        // Verify symbols
        let main_sym = obj.symbols.resolve("main").unwrap();
        assert_eq!(main_sym.section, SymbolSection::Code);

        let buffer_sym = obj.symbols.resolve("buffer").unwrap();
        assert_eq!(buffer_sym.section, SymbolSection::Data);
    }

    #[test]
    fn empty_program() {
        let src = "";
        let obj = assemble(src).unwrap();
        assert!(obj.code.is_empty());
        assert!(obj.data.is_empty());
    }

    #[test]
    fn comments_are_ignored() {
        let src = "\
; This is a comment
    MOVI R1, 5
    ; Another comment
    HALT
";
        let obj = assemble(src).unwrap();
        assert!(!obj.code.is_empty());
    }
}
