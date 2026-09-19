//! Instruction decoder (spec §20): bytes → `DecodedInstruction`.
//!
//! Decoding proceeds in the exact order the spec prescribes: read opcode,
//! look up format, determine length, decode operands, validate operand
//! classes, return a structured instruction. Instruction boundaries are
//! always deterministic because every opcode has exactly one registered
//! format and every field in that format has a fixed width.

use crate::errors::{DecodeError, DecodeErrorKind};
use crate::isa::instruction::DecodedInstruction;
use crate::isa::opcodes::lookup_opcode;
use crate::isa::operand::{Format, Operands};

fn truncated(offset: u64, needed: usize, have: usize) -> DecodeError {
    DecodeError {
        kind: DecodeErrorKind::TruncatedInstruction,
        offset,
        reason: format!("need {} bytes, only {} available", needed, have),
    }
}

/// Decode exactly one instruction starting at `bytes[0]`. `base_offset` is
/// only used to make error messages report the true position in the code
/// section.
pub fn decode_one(bytes: &[u8], base_offset: u64) -> Result<DecodedInstruction, DecodeError> {
    if bytes.is_empty() {
        return Err(truncated(base_offset, 1, 0));
    }
    let opcode = bytes[0];
    let def = lookup_opcode(opcode).ok_or(DecodeError {
        kind: DecodeErrorKind::InvalidOpcode,
        offset: base_offset,
        reason: format!("no instruction registered for opcode {:#04x}", opcode),
    })?;

    let len = def.format.len();
    if bytes.len() < len {
        return Err(truncated(base_offset, len, bytes.len()));
    }
    let body = &bytes[1..len];

    let operands = match def.format {
        Format::None => Operands::None,
        Format::R => Operands::R { rd: body[0] },
        Format::RR => Operands::RR { rd: body[0], rs: body[1] },
        Format::RRR => Operands::RRR { rd: body[0], rs1: body[1], rs2: body[2] },
        Format::RImm64 => {
            let rd = body[0];
            let imm = u64::from_le_bytes(body[1..9].try_into().unwrap());
            Operands::RImm64 { rd, imm }
        }
        Format::RMem => {
            let rd = body[0];
            let base = body[1];
            let disp = i32::from_le_bytes(body[2..6].try_into().unwrap());
            Operands::RMem { rd, base, disp }
        }
        Format::MemR => {
            let base = body[0];
            let disp = i32::from_le_bytes(body[1..5].try_into().unwrap());
            let rs = body[5];
            Operands::MemR { base, disp, rs }
        }
        Format::Cas => {
            let rd = body[0];
            let base = body[1];
            let rs_new = body[2];
            let disp = i32::from_le_bytes(body[3..7].try_into().unwrap());
            Operands::Cas { rd, base, rs_new, disp }
        }
        Format::BranchImm32 => {
            let disp = i32::from_le_bytes(body[0..4].try_into().unwrap());
            Operands::BranchImm32 { disp }
        }
        Format::Imm16 => {
            let imm = u16::from_le_bytes(body[0..2].try_into().unwrap());
            Operands::Imm16 { imm }
        }
    };

    // Register-field validation (spec §20 step 5: "validate operand
    // classes"). Out-of-range register bytes are rejected here rather
    // than deferred to execution, so a decoded instruction is always
    // structurally legal.
    validate_registers(&operands, base_offset)?;

    Ok(DecodedInstruction { mnemonic: def.mnemonic, operands, length: len })
}

fn validate_registers(operands: &Operands, offset: u64) -> Result<(), DecodeError> {
    use crate::isa::constants::NUM_GP_REGISTERS;
    let check = |id: u8| -> Result<(), DecodeError> {
        if (id as usize) < NUM_GP_REGISTERS {
            Ok(())
        } else {
            Err(DecodeError {
                kind: DecodeErrorKind::InvalidRegister,
                offset,
                reason: format!("register id {} is outside 0..{}", id, NUM_GP_REGISTERS),
            })
        }
    };
    match *operands {
        Operands::None | Operands::BranchImm32 { .. } | Operands::Imm16 { .. } => Ok(()),
        Operands::R { rd } => check(rd),
        Operands::RR { rd, rs } => {
            check(rd)?;
            check(rs)
        }
        Operands::RRR { rd, rs1, rs2 } => {
            check(rd)?;
            check(rs1)?;
            check(rs2)
        }
        Operands::RImm64 { rd, .. } => check(rd),
        Operands::RMem { rd, base, .. } => {
            check(rd)?;
            check(base)
        }
        Operands::MemR { base, rs, .. } => {
            check(base)?;
            check(rs)
        }
        Operands::Cas { rd, base, rs_new, .. } => {
            check(rd)?;
            check(base)?;
            check(rs_new)
        }
    }
}

/// Decode an entire byte stream into a sequence of instructions, each
/// tagged with its byte offset. Used by the disassembler and validator.
pub fn decode_all(bytes: &[u8]) -> Result<Vec<(u64, DecodedInstruction)>, DecodeError> {
    let mut out = Vec::new();
    let mut pos: usize = 0;
    while pos < bytes.len() {
        let instr = decode_one(&bytes[pos..], pos as u64)?;
        let len = instr.length;
        out.push((pos as u64, instr));
        pos += len;
    }
    Ok(out)
}
