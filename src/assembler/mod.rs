//! The assembler (spec §16): JXCL assembly source → JXCL binary.
//!
//! Two deterministic passes, as is standard for a label-resolving
//! assembler:
//!
//! 1. **Layout pass** — walk every statement, growing running code/data
//!    offsets from each opcode's *fixed* registry length (never from its
//!    operand values, which aren't evaluated yet), and record every
//!    label's offset within its section. Constants are evaluated
//!    immediately since they can only reference literals/earlier
//!    constants, never labels.
//! 2. **Emission pass** — walk the statements again, this time resolving
//!    every label/constant reference against the now-complete symbol
//!    table and emitting real instructions/data bytes.
//!
//! This module is the client of `isa::opcodes` (for mnemonic/format
//! lookup), `encoding::encoder` (for final byte emission) and `binary`
//! (for the container format) — it does not duplicate any of their
//! knowledge (spec §19).

pub mod lexer;
pub mod parser;

use std::collections::HashMap;

use crate::binary;
use crate::encoding::encoder::encode_all;
use crate::errors::{AssemblerError, AssemblerErrorKind};
use crate::isa::instruction::DecodedInstruction;
use crate::isa::opcodes::{lookup_mnemonic, Mnemonic};
use crate::isa::operand::{Format, Operands};

use parser::{parse_line, DataWidth, InstrNode, OperandNode, Stmt, ValueExpr};

#[derive(Debug, Clone, Copy)]
enum Symbol {
    Code(u64),
    Data(u64),
}

fn err(kind: AssemblerErrorKind, line: usize, reason: impl Into<String>) -> AssemblerError {
    AssemblerError { kind, line, reason: reason.into() }
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
    stmt: Stmt,
}

fn parse_source(source: &str) -> Result<Vec<ParsedLine>, AssemblerError> {
    let mut out = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let tokens = lexer::tokenize(raw_line, line_no)?;
        let (label, stmt) = parse_line(&tokens, line_no)?;
        out.push(ParsedLine { line: line_no, label, stmt });
    }
    Ok(out)
}

/// Assemble complete JXCL assembly source into a full `.jxc` binary file
/// (header + code + data), per the layout/emission passes described above.
pub fn assemble(source: &str) -> Result<Vec<u8>, AssemblerError> {
    let lines = parse_source(source)?;

    // ---- Pass 1: layout ----
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Section {
        Code,
        Data,
    }
    let mut section = Section::Code;
    let mut code_len: u64 = 0;
    let mut data_len: u64 = 0;
    let mut symbols: HashMap<String, Symbol> = HashMap::new();
    let mut consts: HashMap<String, i128> = HashMap::new();
    let mut entry_value: Option<ValueExpr> = None;

    for pl in &lines {
        if let Some(name) = &pl.label {
            if symbols.contains_key(name) {
                return Err(err(AssemblerErrorKind::DuplicateLabel, pl.line, format!("label {:?} redefined", name)));
            }
            let sym = match section {
                Section::Code => Symbol::Code(code_len),
                Section::Data => Symbol::Data(data_len),
            };
            symbols.insert(name.clone(), sym);
        }
        match &pl.stmt {
            Stmt::Empty => {}
            Stmt::SwitchToCode => section = Section::Code,
            Stmt::SwitchToData => section = Section::Data,
            Stmt::ConstDef { name, value } => {
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
            Stmt::EntryDef { value } => entry_value = Some(value.clone()),
            Stmt::Data { width, values } => {
                data_len += data_width_bytes(*width) * values.len() as u64;
            }
            Stmt::Instruction(InstrNode { mnemonic, .. }) => {
                if section != Section::Code {
                    return Err(err(
                        AssemblerErrorKind::UnexpectedToken,
                        pl.line,
                        "instructions are only allowed in the code section (.code)",
                    ));
                }
                let m = Mnemonic::from_text(&mnemonic.to_ascii_uppercase())
                    .ok_or_else(|| err(AssemblerErrorKind::UnknownMnemonic, pl.line, mnemonic.clone()))?;
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
            Some(Symbol::Code(o)) => Ok(*o as i128),
            Some(Symbol::Data(o)) => Ok(total_code_len as i128 + *o as i128),
            None => Err(err(AssemblerErrorKind::UnknownLabel, line, format!("undefined symbol {:?}", name))),
        }
    };
    let eval_value = |v: &ValueExpr, line: usize| -> Result<i128, AssemblerError> {
        match v {
            ValueExpr::Number(n) => Ok(*n),
            ValueExpr::Name(name) => resolve_name(name, line),
        }
    };

    let entry_point: u64 = match &entry_value {
        Some(v) => eval_value(v, 0)? as u64,
        None => 0,
    };

    // ---- Pass 2: emission ----
    let mut section = Section::Code;
    let mut code_pos: u64 = 0;
    let mut code_instrs: Vec<DecodedInstruction> = Vec::new();
    let mut data_bytes: Vec<u8> = Vec::new();

    for pl in &lines {
        match &pl.stmt {
            Stmt::Empty | Stmt::ConstDef { .. } | Stmt::EntryDef { .. } => {}
            Stmt::SwitchToCode => section = Section::Code,
            Stmt::SwitchToData => section = Section::Data,
            Stmt::Data { width, values } => {
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
                        DataWidth::Word => data_bytes.extend_from_slice(&(val as u16).to_le_bytes()),
                        DataWidth::Dword => data_bytes.extend_from_slice(&(val as u32).to_le_bytes()),
                        DataWidth::Qword => data_bytes.extend_from_slice(&(val as u64).to_le_bytes()),
                    }
                }
            }
            Stmt::Instruction(node) => {
                let _ = section; // instructions were already confirmed to be code-only in pass 1
                let instr = emit_instruction(node, code_pos, pl.line, &eval_value)?;
                code_pos += instr.length as u64;
                code_instrs.push(instr);
            }
        }
    }

    let code_bytes = encode_all(&code_instrs)
        .map_err(|e| err(AssemblerErrorKind::MalformedOperand, 0, format!("internal encoder rejection: {}", e)))?;

    Ok(binary::write(entry_point, &code_bytes, &data_bytes))
}

fn expect_reg(op: &OperandNode, line: usize) -> Result<u8, AssemblerError> {
    match op {
        OperandNode::Reg(r) => Ok(*r),
        _ => Err(err(AssemblerErrorKind::MalformedOperand, line, "expected a register operand")),
    }
}

fn expect_mem(op: &OperandNode, line: usize) -> Result<(u8, ValueExpr), AssemblerError> {
    match op {
        OperandNode::Mem { base, disp } => Ok((*base, disp.clone())),
        _ => Err(err(AssemblerErrorKind::MalformedOperand, line, "expected a memory operand [base+disp]")),
    }
}

fn expect_value(op: &OperandNode, line: usize) -> Result<ValueExpr, AssemblerError> {
    match op {
        OperandNode::Value(v) => Ok(v.clone()),
        _ => Err(err(AssemblerErrorKind::MalformedOperand, line, "expected an immediate, label, or constant")),
    }
}

fn check_arity(node: &InstrNode, expected: usize, line: usize) -> Result<(), AssemblerError> {
    if node.operands.len() != expected {
        return Err(err(
            AssemblerErrorKind::OperandCountMismatch,
            line,
            format!("{} expects {} operand(s), got {}", node.mnemonic, expected, node.operands.len()),
        ));
    }
    Ok(())
}

fn emit_instruction(
    node: &InstrNode,
    code_pos: u64,
    line: usize,
    eval_value: &dyn Fn(&ValueExpr, usize) -> Result<i128, AssemblerError>,
) -> Result<DecodedInstruction, AssemblerError> {
    let mnemonic = Mnemonic::from_text(&node.mnemonic.to_ascii_uppercase())
        .ok_or_else(|| err(AssemblerErrorKind::UnknownMnemonic, line, node.mnemonic.clone()))?;
    let def = lookup_mnemonic(mnemonic);
    let pc_after_fetch = code_pos + def.format.len() as u64;

    let operands = match def.format {
        Format::None => {
            check_arity(node, 0, line)?;
            Operands::None
        }
        Format::R => {
            check_arity(node, 1, line)?;
            Operands::R { rd: expect_reg(&node.operands[0], line)? }
        }
        Format::RR => {
            check_arity(node, 2, line)?;
            Operands::RR {
                rd: expect_reg(&node.operands[0], line)?,
                rs: expect_reg(&node.operands[1], line)?,
            }
        }
        Format::RRR => {
            check_arity(node, 3, line)?;
            Operands::RRR {
                rd: expect_reg(&node.operands[0], line)?,
                rs1: expect_reg(&node.operands[1], line)?,
                rs2: expect_reg(&node.operands[2], line)?,
            }
        }
        Format::RImm64 => {
            check_arity(node, 2, line)?;
            let rd = expect_reg(&node.operands[0], line)?;
            let v = eval_value(&expect_value(&node.operands[1], line)?, line)?;
            if !fits_width(v, 64) {
                return Err(err(AssemblerErrorKind::ImmediateOutOfRange, line, format!("{} does not fit in 64 bits", v)));
            }
            Operands::RImm64 { rd, imm: v as u64 }
        }
        Format::RMem => {
            check_arity(node, 2, line)?;
            let rd = expect_reg(&node.operands[0], line)?;
            let (base, disp_expr) = expect_mem(&node.operands[1], line)?;
            let disp = eval_value(&disp_expr, line)?;
            if !fits_width(disp, 32) {
                return Err(err(AssemblerErrorKind::ImmediateOutOfRange, line, format!("displacement {} does not fit in 32 bits", disp)));
            }
            Operands::RMem { rd, base, disp: disp as i32 }
        }
        Format::MemR => {
            check_arity(node, 2, line)?;
            let (base, disp_expr) = expect_mem(&node.operands[0], line)?;
            let disp = eval_value(&disp_expr, line)?;
            if !fits_width(disp, 32) {
                return Err(err(AssemblerErrorKind::ImmediateOutOfRange, line, format!("displacement {} does not fit in 32 bits", disp)));
            }
            let rs = expect_reg(&node.operands[1], line)?;
            Operands::MemR { base, disp: disp as i32, rs }
        }
        Format::Cas => {
            check_arity(node, 3, line)?;
            let rd = expect_reg(&node.operands[0], line)?;
            let (base, disp_expr) = expect_mem(&node.operands[1], line)?;
            let disp = eval_value(&disp_expr, line)?;
            if !fits_width(disp, 32) {
                return Err(err(AssemblerErrorKind::ImmediateOutOfRange, line, format!("displacement {} does not fit in 32 bits", disp)));
            }
            let rs_new = expect_reg(&node.operands[2], line)?;
            Operands::Cas { rd, base, rs_new, disp: disp as i32 }
        }
        Format::BranchImm32 => {
            check_arity(node, 1, line)?;
            let target = eval_value(&expect_value(&node.operands[0], line)?, line)?;
            let disp = target - pc_after_fetch as i128;
            if !fits_width(disp, 32) {
                return Err(err(
                    AssemblerErrorKind::ImmediateOutOfRange,
                    line,
                    format!("branch target {} is out of ±2GiB range from {}", target, pc_after_fetch),
                ));
            }
            Operands::BranchImm32 { disp: disp as i32 }
        }
        Format::Imm16 => {
            check_arity(node, 1, line)?;
            let v = eval_value(&expect_value(&node.operands[0], line)?, line)?;
            if !fits_width(v, 16) {
                return Err(err(AssemblerErrorKind::ImmediateOutOfRange, line, format!("{} does not fit in 16 bits", v)));
            }
            Operands::Imm16 { imm: v as u16 }
        }
    };

    Ok(DecodedInstruction::new(mnemonic, operands))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator;

    #[test]
    fn assembles_and_validates_simple_program() {
        let src = "\
start:
    MOVI R1, 10
    MOVI R2, 20
    ADD  R1, R2
    HALT
";
        let bin = assemble(src).unwrap();
        let instrs = validator::validate(&bin).unwrap();
        assert_eq!(instrs.len(), 4);
    }

    #[test]
    fn resolves_forward_branch_label() {
        let src = "\
    JMP skip
    HALT
skip:
    HALT
";
        let bin = assemble(src).unwrap();
        let program = binary::parse(&bin).unwrap();
        // JMP (5 bytes) + HALT (1 byte) = target offset 6.
        let decoded = crate::encoding::decoder::decode_all(program.code).unwrap();
        match decoded[0].1.operands {
            Operands::BranchImm32 { disp } => assert_eq!(disp, 1), // pc_after_fetch=5, target=6
            other => panic!("unexpected operands {:?}", other),
        }
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
    fn data_section_round_trips_through_qword_label() {
        let src = "\
    MOVI R1, buffer
    HALT
.data
buffer:
    .qword 42
";
        let bin = assemble(src).unwrap();
        let program = binary::parse(&bin).unwrap();
        assert_eq!(program.data, &42u64.to_le_bytes());
        // buffer's absolute address equals the code section length.
        let decoded = crate::encoding::decoder::decode_all(program.code).unwrap();
        match decoded[0].1.operands {
            Operands::RImm64 { imm, .. } => assert_eq!(imm, program.header.code_size),
            other => panic!("unexpected operands {:?}", other),
        }
    }

    #[test]
    fn immediate_out_of_range_is_rejected() {
        let err = assemble(".data\n.byte 300\n").unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::ImmediateOutOfRange);
    }
}
