// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Instruction encoder (spec §21): `DecodedInstruction` -> bytes.
//!
//! Extracted from `jxcl/src/encoding/encoder.rs`. The encoder is the
//! strict inverse of `jxcl-decoding` (spec §18's round-trip invariant:
//! `encode(decode(bytes)) == bytes` for every canonical encoding). It
//! rejects anything the registry doesn't sanction -- it never silently
//! truncates or reinterprets an operand.
//!
//! ## Deviation from the pre-expansion source
//!
//! The pre-expansion encoder also range-checked every register byte
//! against `NUM_GP_REGISTERS`. Per `docs/crates.toml`, this crate
//! depends on `jxcl-instructions`/`jxcl-bitops`/`jxcl-endian`/
//! `jxcl-bytes`/`jxcl-errors` only -- not `jxcl-constants` or
//! `jxcl-registers` -- so register-range legality is not this crate's
//! concern; it belongs to `jxcl-instruction-validation` (the crate this
//! batch adds specifically to be "the single source of truth for 'is
//! this a legal instruction'"). What this crate still owns and checks
//! is *structural* legality: does the given operand shape match what
//! the opcode registry declares for this mnemonic (via
//! `jxcl-instructions::format_matches_registry`).
#![forbid(unsafe_code)]

use jxcl_bytes::ByteCursorMut;
use jxcl_errors::{EncodeError, EncodeErrorKind};
use jxcl_instructions::{format_matches_registry, opcode_byte, DecodedInstruction, Operands};

/// Encode one instruction, appending its bytes to `out`. Returns the
/// number of bytes written.
pub fn encode(instr: &DecodedInstruction, out: &mut Vec<u8>) -> Result<usize, EncodeError> {
    if !format_matches_registry(instr) {
        return Err(EncodeError {
            kind: EncodeErrorKind::InvalidOperandType,
            reason: format!(
                "operand shape {:?} does not match the format registered for {:?}",
                instr.operands.format(),
                instr.mnemonic
            ),
        });
    }

    let mut cursor = ByteCursorMut::from_vec(std::mem::take(out));
    let start = cursor.position();

    cursor
        .write_u8(opcode_byte(instr.mnemonic))
        .expect("appending to a growable buffer never runs out of bounds");

    write_operands(&mut cursor, &instr.operands)
        .expect("appending to a growable buffer never runs out of bounds");

    let written = cursor.position() - start;
    debug_assert_eq!(written, instr.length);
    *out = cursor.into_vec();
    Ok(written)
}

fn write_operands(
    cursor: &mut ByteCursorMut,
    operands: &Operands,
) -> Result<(), jxcl_errors::Error> {
    match *operands {
        Operands::None => Ok(()),
        Operands::R { rd } => cursor.write_u8(rd.get()),
        Operands::RR { rd, rs } => {
            cursor.write_u8(rd.get())?;
            cursor.write_u8(rs.get())
        }
        Operands::RRR { rd, rs1, rs2 } => {
            cursor.write_u8(rd.get())?;
            cursor.write_u8(rs1.get())?;
            cursor.write_u8(rs2.get())
        }
        Operands::RImm64 { rd, imm } => {
            cursor.write_u8(rd.get())?;
            cursor.write_u64(imm)
        }
        Operands::RMem { rd, base, disp } => {
            cursor.write_u8(rd.get())?;
            cursor.write_u8(base.get())?;
            write_disp32(cursor, disp)
        }
        Operands::MemR { base, disp, rs } => {
            cursor.write_u8(base.get())?;
            write_disp32(cursor, disp)?;
            cursor.write_u8(rs.get())
        }
        Operands::Cas {
            rd,
            base,
            rs_new,
            disp,
        } => {
            cursor.write_u8(rd.get())?;
            cursor.write_u8(base.get())?;
            cursor.write_u8(rs_new.get())?;
            write_disp32(cursor, disp)
        }
        Operands::BranchImm32 { disp } => write_disp32(cursor, disp),
        Operands::Imm16 { imm } => cursor.write_u16(imm),
    }
}

/// Write a signed 32-bit displacement, verifying via
/// `jxcl-bitops::sign_extend` (rather than trusting `i32`'s own
/// bit pattern blindly) that the 32 bits written really do decode back
/// to the same signed value -- the same oracle `jxcl-decoding` uses when
/// reading a displacement back, so an encode/decode disagreement here
/// would show up as a debug-mode assertion failure rather than a silent
/// round-trip bug.
fn write_disp32(cursor: &mut ByteCursorMut, disp: i32) -> Result<(), jxcl_errors::Error> {
    debug_assert_eq!(
        jxcl_bitops::sign_extend(disp as u32 as u64, 32) as i32,
        disp,
        "disp32 must round-trip through 32-bit sign extension"
    );
    cursor.write_i32(disp)
}

/// Encode a full instruction stream.
pub fn encode_all(instrs: &[DecodedInstruction]) -> Result<Vec<u8>, EncodeError> {
    let mut out = Vec::with_capacity(instrs.len() * 4);
    for instr in instrs {
        encode(instr, &mut out)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_instructions::{Instruction, Mnemonic, Register};

    #[test]
    fn encodes_none_format_as_single_opcode_byte() {
        let instr = Instruction::new(Mnemonic::Halt, Operands::None);
        let mut out = Vec::new();
        let n = encode(&instr, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out, vec![jxcl_instructions::opcode_byte(Mnemonic::Halt)]);
    }

    #[test]
    fn encodes_rr_format() {
        let instr = Instruction::new(
            Mnemonic::Add,
            Operands::RR {
                rd: Register::new(1),
                rs: Register::new(2),
            },
        );
        let mut out = Vec::new();
        encode(&instr, &mut out).unwrap();
        assert_eq!(
            out,
            vec![jxcl_instructions::opcode_byte(Mnemonic::Add), 1, 2]
        );
    }

    #[test]
    fn encodes_rimm64_little_endian() {
        let instr = Instruction::new(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: Register::new(3),
                imm: 0x0102_0304_0506_0708,
            },
        );
        let mut out = Vec::new();
        encode(&instr, &mut out).unwrap();
        let mut expected = vec![jxcl_instructions::opcode_byte(Mnemonic::Movi), 3];
        expected.extend_from_slice(&0x0102_0304_0506_0708u64.to_le_bytes());
        assert_eq!(out, expected);
    }

    #[test]
    fn encodes_negative_displacement_little_endian() {
        let instr = Instruction::new(Mnemonic::Jmp, Operands::BranchImm32 { disp: -16 });
        let mut out = Vec::new();
        encode(&instr, &mut out).unwrap();
        let mut expected = vec![jxcl_instructions::opcode_byte(Mnemonic::Jmp)];
        expected.extend_from_slice(&(-16i32).to_le_bytes());
        assert_eq!(out, expected);
    }

    #[test]
    fn encodes_cas_format_in_field_order() {
        let instr = Instruction::new(
            Mnemonic::Cas,
            Operands::Cas {
                rd: Register::new(1),
                base: Register::new(2),
                rs_new: Register::new(3),
                disp: 7,
            },
        );
        let mut out = Vec::new();
        encode(&instr, &mut out).unwrap();
        let mut expected = vec![jxcl_instructions::opcode_byte(Mnemonic::Cas), 1, 2, 3];
        expected.extend_from_slice(&7i32.to_le_bytes());
        assert_eq!(out, expected);
    }

    #[test]
    fn rejects_operand_shape_mismatched_with_registry() {
        // ADD is registered RR, not R.
        let instr = Instruction::new(
            Mnemonic::Add,
            Operands::R {
                rd: Register::new(1),
            },
        );
        let mut out = Vec::new();
        let err = encode(&instr, &mut out).unwrap_err();
        assert_eq!(err.kind, EncodeErrorKind::InvalidOperandType);
        // Nothing was written on failure.
        assert!(out.is_empty());
    }

    #[test]
    fn encode_all_concatenates_every_instruction() {
        let program = [
            Instruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd: Register::new(1),
                    imm: 1,
                },
            ),
            Instruction::new(Mnemonic::Halt, Operands::None),
        ];
        let bytes = encode_all(&program).unwrap();
        assert_eq!(bytes.len(), 10 + 1);
    }

    #[test]
    fn encode_appends_without_disturbing_existing_bytes() {
        let mut out = vec![0xAA, 0xBB];
        let instr = Instruction::new(Mnemonic::Nop, Operands::None);
        encode(&instr, &mut out).unwrap();
        assert_eq!(
            out,
            vec![0xAA, 0xBB, jxcl_instructions::opcode_byte(Mnemonic::Nop)]
        );
    }
}
