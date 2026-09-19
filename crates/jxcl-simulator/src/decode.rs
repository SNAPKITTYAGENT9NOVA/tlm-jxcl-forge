//! Instruction decoder for this crate's embedded ISA subset (`isa`
//! module): bytes -> `DecodedInstruction`, ported from
//! `crates/jxcl/src/encoding/decoder.rs`'s algorithm (read opcode, look
//! up format, determine length, decode operands, validate register
//! fields) but restricted to the opcodes `isa::DEFS` actually lists.

use crate::isa::{lookup_opcode, Format, Mnemonic, NUM_GP_REGISTERS};

pub type RegId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operands {
    None,
    R { rd: RegId },
    RR { rd: RegId, rs: RegId },
    RImm64 { rd: RegId, imm: u64 },
    RMem { rd: RegId, base: RegId, disp: i32 },
    MemR { base: RegId, disp: i32, rs: RegId },
    BranchImm32 { disp: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub mnemonic: Mnemonic,
    pub operands: Operands,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeErrorKind {
    InvalidOpcode,
    TruncatedInstruction,
    InvalidRegister,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub kind: DecodeErrorKind,
    pub offset: u64,
    pub reason: String,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "decode error at offset {:#x}: {:?}: {}",
            self.offset, self.kind, self.reason
        )
    }
}
impl std::error::Error for DecodeError {}

fn truncated(offset: u64, needed: usize, have: usize) -> DecodeError {
    DecodeError {
        kind: DecodeErrorKind::TruncatedInstruction,
        offset,
        reason: format!("need {needed} bytes, only {have} available"),
    }
}

fn check_register(id: u8, offset: u64) -> Result<(), DecodeError> {
    if (id as usize) < NUM_GP_REGISTERS {
        Ok(())
    } else {
        Err(DecodeError {
            kind: DecodeErrorKind::InvalidRegister,
            offset,
            reason: format!("register id {id} is outside 0..{NUM_GP_REGISTERS}"),
        })
    }
}

/// Decode exactly one instruction starting at `bytes[0]`. Never panics
/// and never reads past `bytes`, for any input including the empty
/// slice or malformed opcode bytes (this is the exact invariant
/// `jxcl-fuzz`'s ground-truth counterpart, `jxcl/tests/fuzz_decoder.rs`,
/// checks against the real decoder).
pub fn decode_one(bytes: &[u8], base_offset: u64) -> Result<DecodedInstruction, DecodeError> {
    if bytes.is_empty() {
        return Err(truncated(base_offset, 1, 0));
    }
    let opcode = bytes[0];
    let def = lookup_opcode(opcode).ok_or(DecodeError {
        kind: DecodeErrorKind::InvalidOpcode,
        offset: base_offset,
        reason: format!("no instruction registered for opcode {opcode:#04x}"),
    })?;

    let len = def.format.len();
    if bytes.len() < len {
        return Err(truncated(base_offset, len, bytes.len()));
    }
    let body = &bytes[1..len];

    let operands = match def.format {
        Format::None => Operands::None,
        Format::R => Operands::R { rd: body[0] },
        Format::RR => Operands::RR {
            rd: body[0],
            rs: body[1],
        },
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
        Format::BranchImm32 => {
            let disp = i32::from_le_bytes(body[0..4].try_into().unwrap());
            Operands::BranchImm32 { disp }
        }
    };

    match operands {
        Operands::None | Operands::BranchImm32 { .. } => {}
        Operands::R { rd } => check_register(rd, base_offset)?,
        Operands::RR { rd, rs } => {
            check_register(rd, base_offset)?;
            check_register(rs, base_offset)?;
        }
        Operands::RImm64 { rd, .. } => check_register(rd, base_offset)?,
        Operands::RMem { rd, base, .. } => {
            check_register(rd, base_offset)?;
            check_register(base, base_offset)?;
        }
        Operands::MemR { base, rs, .. } => {
            check_register(base, base_offset)?;
            check_register(rs, base_offset)?;
        }
    }

    Ok(DecodedInstruction {
        mnemonic: def.mnemonic,
        operands,
        length: len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_truncated() {
        let e = decode_one(&[], 0).unwrap_err();
        assert_eq!(e.kind, DecodeErrorKind::TruncatedInstruction);
    }

    #[test]
    fn unknown_opcode_is_invalid() {
        let e = decode_one(&[0xEE], 0).unwrap_err();
        assert_eq!(e.kind, DecodeErrorKind::InvalidOpcode);
    }

    #[test]
    fn movi_decodes_register_and_immediate() {
        let mut bytes = vec![0x02, 3];
        bytes.extend_from_slice(&42u64.to_le_bytes());
        let instr = decode_one(&bytes, 0).unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Movi);
        assert_eq!(instr.operands, Operands::RImm64 { rd: 3, imm: 42 });
        assert_eq!(instr.length, 10);
    }

    #[test]
    fn out_of_range_register_is_rejected() {
        let mut bytes = vec![0x02, 200];
        bytes.extend_from_slice(&0u64.to_le_bytes());
        let e = decode_one(&bytes, 0).unwrap_err();
        assert_eq!(e.kind, DecodeErrorKind::InvalidRegister);
    }

    #[test]
    fn truncated_movi_is_rejected_not_panicking() {
        let e = decode_one(&[0x02, 3, 1, 2], 0).unwrap_err();
        assert_eq!(e.kind, DecodeErrorKind::TruncatedInstruction);
    }
}
