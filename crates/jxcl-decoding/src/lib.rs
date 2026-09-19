//! Instruction decoder (spec §20): bytes -> `DecodedInstruction`.
//!
//! Extracted from `jxcl/src/encoding/decoder.rs`. Decoding proceeds in
//! the order the spec prescribes: read opcode, look up format,
//! determine length, decode operands, return a structured instruction.
//! Instruction boundaries are always deterministic because every opcode
//! has exactly one registered format and every field in that format has
//! a fixed width.
//!
//! ## Deviation from the pre-expansion source
//!
//! The pre-expansion decoder also range-checked every decoded register
//! byte against `NUM_GP_REGISTERS`, returning `DecodeErrorKind::
//! InvalidRegister` on failure. Per `docs/crates.toml`, this crate
//! depends on `jxcl-instructions`/`jxcl-bitops`/`jxcl-endian`/
//! `jxcl-bytes`/`jxcl-errors` only -- not `jxcl-constants` or
//! `jxcl-registers` -- so register-range legality is not this crate's
//! concern; it belongs to `jxcl-instruction-validation` (see that
//! crate's module doc: "the single source of truth for 'is this a legal
//! instruction', so the decoder and jxcl-validator never diverge"). This
//! crate decodes every register field byte structurally (any `0..=255`
//! value decodes without error); a caller that needs semantic legality
//! runs the result through `jxcl-instruction-validation::validate_instruction`.
#![forbid(unsafe_code)]

use jxcl_bytes::ByteCursor;
use jxcl_errors::{DecodeError, DecodeErrorKind};
use jxcl_instructions::{
    instruction_shape_for_opcode, DecodedInstruction, Format, Operands, Register,
};

fn truncated(offset: u64, needed: usize, have: usize) -> DecodeError {
    DecodeError {
        kind: DecodeErrorKind::TruncatedInstruction,
        offset: jxcl_types::Address(offset),
        reason: format!("need {} bytes, only {} available", needed, have),
    }
}

fn read_register(cursor: &mut ByteCursor) -> Register {
    Register::new(
        cursor
            .read_u8()
            .expect("caller already validated the instruction's full length"),
    )
}

/// Read a signed 32-bit displacement, decoding it via
/// `jxcl-bitops::sign_extend` rather than relying on `i32::from_le_bytes`'s
/// own (equivalent, but separately-implemented) sign handling -- the
/// same shared primitive `jxcl-encoding` cross-checks its writes
/// against, so encoder and decoder provably agree on what "sign extend
/// the low 32 bits" means.
fn read_disp32(cursor: &mut ByteCursor) -> i32 {
    let raw = cursor
        .read_u32()
        .expect("caller already validated the instruction's full length");
    jxcl_bitops::sign_extend(raw as u64, 32) as i32
}

/// Decode exactly one instruction starting at `bytes[0]`. `base_offset`
/// is only used to make error messages report the true position in the
/// code section.
pub fn decode_one(bytes: &[u8], base_offset: u64) -> Result<DecodedInstruction, DecodeError> {
    if bytes.is_empty() {
        return Err(truncated(base_offset, 1, 0));
    }
    let opcode = bytes[0];
    let (mnemonic, format) = instruction_shape_for_opcode(opcode).ok_or(DecodeError {
        kind: DecodeErrorKind::InvalidOpcode,
        offset: jxcl_types::Address(base_offset),
        reason: format!("no instruction registered for opcode {:#04x}", opcode),
    })?;

    let len = format.len();
    if bytes.len() < len {
        return Err(truncated(base_offset, len, bytes.len()));
    }
    let mut cursor = ByteCursor::new(&bytes[1..len]);

    let operands = match format {
        Format::None => Operands::None,
        Format::R => Operands::R {
            rd: read_register(&mut cursor),
        },
        Format::RR => {
            let rd = read_register(&mut cursor);
            let rs = read_register(&mut cursor);
            Operands::RR { rd, rs }
        }
        Format::RRR => {
            let rd = read_register(&mut cursor);
            let rs1 = read_register(&mut cursor);
            let rs2 = read_register(&mut cursor);
            Operands::RRR { rd, rs1, rs2 }
        }
        Format::RImm64 => {
            let rd = read_register(&mut cursor);
            let imm = cursor
                .read_u64()
                .expect("caller already validated the instruction's full length");
            Operands::RImm64 { rd, imm }
        }
        Format::RMem => {
            let rd = read_register(&mut cursor);
            let base = read_register(&mut cursor);
            let disp = read_disp32(&mut cursor);
            Operands::RMem { rd, base, disp }
        }
        Format::MemR => {
            let base = read_register(&mut cursor);
            let disp = read_disp32(&mut cursor);
            let rs = read_register(&mut cursor);
            Operands::MemR { base, disp, rs }
        }
        Format::Cas => {
            let rd = read_register(&mut cursor);
            let base = read_register(&mut cursor);
            let rs_new = read_register(&mut cursor);
            let disp = read_disp32(&mut cursor);
            Operands::Cas {
                rd,
                base,
                rs_new,
                disp,
            }
        }
        Format::BranchImm32 => Operands::BranchImm32 {
            disp: read_disp32(&mut cursor),
        },
        Format::Imm16 => Operands::Imm16 {
            imm: cursor
                .read_u16()
                .expect("caller already validated the instruction's full length"),
        },
    };

    Ok(DecodedInstruction {
        mnemonic,
        operands,
        length: len,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_encoding::encode_all;
    use jxcl_instructions::{opcode_byte, Instruction, Mnemonic};

    #[test]
    fn decodes_none_format() {
        let bytes = [opcode_byte(Mnemonic::Halt)];
        let instr = decode_one(&bytes, 0).unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Halt);
        assert_eq!(instr.operands, Operands::None);
        assert_eq!(instr.length, 1);
    }

    #[test]
    fn decodes_rr_format() {
        let bytes = [opcode_byte(Mnemonic::Add), 5, 6];
        let instr = decode_one(&bytes, 0).unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Add);
        assert_eq!(
            instr.operands,
            Operands::RR {
                rd: Register::new(5),
                rs: Register::new(6)
            }
        );
    }

    #[test]
    fn decodes_rimm64_little_endian() {
        let mut bytes = vec![opcode_byte(Mnemonic::Movi), 2];
        bytes.extend_from_slice(&0x0102_0304_0506_0708u64.to_le_bytes());
        let instr = decode_one(&bytes, 0).unwrap();
        assert_eq!(
            instr.operands,
            Operands::RImm64 {
                rd: Register::new(2),
                imm: 0x0102_0304_0506_0708
            }
        );
    }

    #[test]
    fn decodes_negative_displacement() {
        let mut bytes = vec![opcode_byte(Mnemonic::Jmp)];
        bytes.extend_from_slice(&(-16i32).to_le_bytes());
        let instr = decode_one(&bytes, 0).unwrap();
        assert_eq!(instr.operands, Operands::BranchImm32 { disp: -16 });
    }

    #[test]
    fn decodes_cas_field_order() {
        let mut bytes = vec![opcode_byte(Mnemonic::Cas), 1, 2, 3];
        bytes.extend_from_slice(&9i32.to_le_bytes());
        let instr = decode_one(&bytes, 0).unwrap();
        assert_eq!(
            instr.operands,
            Operands::Cas {
                rd: Register::new(1),
                base: Register::new(2),
                rs_new: Register::new(3),
                disp: 9
            }
        );
    }

    #[test]
    fn rejects_unassigned_opcode() {
        let bytes = [0xFF];
        let err = decode_one(&bytes, 0).unwrap_err();
        assert_eq!(err.kind, DecodeErrorKind::InvalidOpcode);
    }

    #[test]
    fn rejects_empty_input() {
        let err = decode_one(&[], 5).unwrap_err();
        assert_eq!(err.kind, DecodeErrorKind::TruncatedInstruction);
        assert_eq!(err.offset, jxcl_types::Address(5));
    }

    #[test]
    fn rejects_truncated_multi_byte_instruction() {
        // MOVI needs 10 bytes total; give it 3.
        let bytes = [opcode_byte(Mnemonic::Movi), 0, 0];
        let err = decode_one(&bytes, 100).unwrap_err();
        assert_eq!(err.kind, DecodeErrorKind::TruncatedInstruction);
        assert_eq!(err.offset, jxcl_types::Address(100));
    }

    #[test]
    fn decode_all_walks_every_instruction_boundary() {
        let bytes = [
            opcode_byte(Mnemonic::Nop),
            opcode_byte(Mnemonic::Add),
            1,
            2,
            opcode_byte(Mnemonic::Halt),
        ];
        let instrs = decode_all(&bytes).unwrap();
        assert_eq!(instrs.len(), 3);
        assert_eq!(
            instrs[0],
            (0, Instruction::new(Mnemonic::Nop, Operands::None))
        );
        assert_eq!(
            instrs[1],
            (
                1,
                Instruction::new(
                    Mnemonic::Add,
                    Operands::RR {
                        rd: Register::new(1),
                        rs: Register::new(2)
                    }
                )
            )
        );
        assert_eq!(
            instrs[2],
            (4, Instruction::new(Mnemonic::Halt, Operands::None))
        );
    }

    #[test]
    fn round_trips_through_encode_for_every_registered_shape() {
        let rd = Register::new(4);
        let program = [
            Instruction::new(Mnemonic::Nop, Operands::None),
            Instruction::new(Mnemonic::Push, Operands::R { rd }),
            Instruction::new(Mnemonic::Add, Operands::RR { rd, rs: rd }),
            Instruction::new(
                Mnemonic::Xor3,
                Operands::RRR {
                    rd,
                    rs1: rd,
                    rs2: rd,
                },
            ),
            Instruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd,
                    imm: 0xDEAD_BEEF,
                },
            ),
            Instruction::new(
                Mnemonic::Load,
                Operands::RMem {
                    rd,
                    base: rd,
                    disp: -4,
                },
            ),
            Instruction::new(
                Mnemonic::Store,
                Operands::MemR {
                    base: rd,
                    disp: 4,
                    rs: rd,
                },
            ),
            Instruction::new(
                Mnemonic::Cas,
                Operands::Cas {
                    rd,
                    base: rd,
                    rs_new: rd,
                    disp: -1,
                },
            ),
            Instruction::new(Mnemonic::Jmp, Operands::BranchImm32 { disp: 100 }),
            Instruction::new(Mnemonic::Trap, Operands::Imm16 { imm: 7 }),
        ];
        let bytes = encode_all(&program).unwrap();
        let decoded = decode_all(&bytes).unwrap();
        let decoded_instrs: Vec<Instruction> = decoded.into_iter().map(|(_, i)| i).collect();
        assert_eq!(decoded_instrs, program);

        // And the other direction: re-encoding the decoded stream
        // reproduces the exact same bytes (spec §18's round-trip
        // invariant).
        let re_encoded = encode_all(&decoded_instrs).unwrap();
        assert_eq!(re_encoded, bytes);
    }
}
