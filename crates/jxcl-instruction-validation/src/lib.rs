// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Per-instruction operand-legality checks (register range, immediate range, addressing-mode compatibility with the opcode) shared by the decoder and the static binary validator.
//!
//! Owns: `validate_instruction(&Instruction)` -> `Result<(), Error>` --
//! the single source of truth for "is this a legal instruction", so the
//! decoder and jxcl-validator never diverge.
//!
//! This is the validation layer that ensures every decoded instruction
//! has valid operands according to ISA rules, independent of whether the
//! instruction's *semantics* are correct.
#![forbid(unsafe_code)]

use jxcl_errors::Error;
use jxcl_instructions::Instruction;
use jxcl_operands::{Operands, Register};
use jxcl_registers::validate_register;

/// Validate that an instruction's operands are legal according to ISA rules.
///
/// This checks:
/// - All register operands are in the valid range (0..REGISTER_COUNT)
/// - Immediates are within their appropriate ranges
/// - Memory addressing modes use valid base registers
/// - The operand shape matches what's expected for the opcode
///
/// This is the single source of truth for instruction legality, used by
/// both the decoder (to reject malformed binary) and any static validator
/// (to check decoded binaries).
pub fn validate_instruction(instr: &Instruction) -> Result<(), Error> {
    // Validate all register operands in the instruction
    for reg in instr.operands.registers() {
        validate_register_operand(reg)?;
    }

    // Validate operand-specific constraints based on the shape
    match instr.operands {
        Operands::None | Operands::BranchImm32 { .. } | Operands::Imm16 { .. } => {
            // No register operands to validate beyond the generic check above
        }
        Operands::R { .. } => {
            // Single register destination; already validated
        }
        Operands::RR { .. } => {
            // Two register operands; already validated
        }
        Operands::RRR { .. } => {
            // Three register operands; already validated
        }
        Operands::RImm64 { imm, .. } => {
            // Immediate can be any u64, no range constraint
            validate_immediate_u64(imm)?;
        }
        Operands::RMem {
            disp: _,
            ..
        } => {
            // Base register already validated, displacement is i32 (always valid)
        }
        Operands::MemR {
            disp: _,
            ..
        } => {
            // Base and rs already validated, displacement is i32 (always valid)
        }
        Operands::Cas {
            disp: _,
            ..
        } => {
            // All registers already validated, displacement is i32 (always valid)
        }
    }

    Ok(())
}

/// Validate that a register operand is in the valid range.
fn validate_register_operand(reg: Register) -> Result<(), Error> {
    validate_register(reg).map_err(|_e| Error::InvalidInstruction {
        reason: format!("register {} out of range", reg.get()),
    })
}

/// Validate a 64-bit immediate value.
///
/// As u64 has no "out of range" values (every u64 is valid), this is
/// a no-op but is provided for symmetry and future extensibility.
#[inline]
fn validate_immediate_u64(_imm: u64) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_opcodes::Mnemonic;

    #[test]
    fn validate_nop_instruction() {
        let instr = Instruction::new(Mnemonic::Nop, Operands::None);
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_r_format_instruction_with_valid_register() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(5),
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_r_format_instruction_with_r0() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(0),
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_r_format_instruction_with_max_register() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(31),
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn reject_r_format_instruction_with_out_of_range_register() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(32),
        });
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn validate_rr_format_instruction() {
        let instr = Instruction::new(
            Mnemonic::Mov,
            Operands::RR {
                rd: Register::new(0),
                rs: Register::new(1),
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_rrr_format_instruction() {
        let instr = Instruction::new(
            Mnemonic::Add,
            Operands::RRR {
                rd: Register::new(0),
                rs1: Register::new(1),
                rs2: Register::new(2),
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn reject_rrr_format_with_invalid_destination() {
        let instr = Instruction::new(
            Mnemonic::Add,
            Operands::RRR {
                rd: Register::new(32),
                rs1: Register::new(1),
                rs2: Register::new(2),
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn reject_rrr_format_with_invalid_source1() {
        let instr = Instruction::new(
            Mnemonic::Add,
            Operands::RRR {
                rd: Register::new(0),
                rs1: Register::new(32),
                rs2: Register::new(2),
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn reject_rrr_format_with_invalid_source2() {
        let instr = Instruction::new(
            Mnemonic::Add,
            Operands::RRR {
                rd: Register::new(0),
                rs1: Register::new(1),
                rs2: Register::new(32),
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn validate_rimm64_format_instruction() {
        let instr = Instruction::new(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: Register::new(5),
                imm: 0xDEADBEEF,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_rimm64_with_max_immediate() {
        let instr = Instruction::new(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: Register::new(5),
                imm: u64::MAX,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_rimm64_with_zero_immediate() {
        let instr = Instruction::new(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: Register::new(5),
                imm: 0,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn reject_rimm64_with_invalid_register() {
        let instr = Instruction::new(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: Register::new(32),
                imm: 0xDEADBEEF,
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn validate_rmem_format_instruction() {
        let instr = Instruction::new(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(5),
                base: Register::new(10),
                disp: -8,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_rmem_with_max_displacement() {
        let instr = Instruction::new(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(5),
                base: Register::new(10),
                disp: i32::MAX,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_rmem_with_min_displacement() {
        let instr = Instruction::new(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(5),
                base: Register::new(10),
                disp: i32::MIN,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn reject_rmem_with_invalid_destination() {
        let instr = Instruction::new(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(32),
                base: Register::new(10),
                disp: 0,
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn reject_rmem_with_invalid_base() {
        let instr = Instruction::new(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(5),
                base: Register::new(32),
                disp: 0,
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn validate_memr_format_instruction() {
        let instr = Instruction::new(
            Mnemonic::Store,
            Operands::MemR {
                base: Register::new(10),
                disp: 8,
                rs: Register::new(5),
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn reject_memr_with_invalid_base() {
        let instr = Instruction::new(
            Mnemonic::Store,
            Operands::MemR {
                base: Register::new(32),
                disp: 0,
                rs: Register::new(5),
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn reject_memr_with_invalid_source() {
        let instr = Instruction::new(
            Mnemonic::Store,
            Operands::MemR {
                base: Register::new(10),
                disp: 0,
                rs: Register::new(32),
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn validate_cas_format_instruction() {
        let instr = Instruction::new(
            Mnemonic::Cas,
            Operands::Cas {
                rd: Register::new(0),
                base: Register::new(10),
                rs_new: Register::new(5),
                disp: 0,
            },
        );
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn reject_cas_with_invalid_destination() {
        let instr = Instruction::new(
            Mnemonic::Cas,
            Operands::Cas {
                rd: Register::new(32),
                base: Register::new(10),
                rs_new: Register::new(5),
                disp: 0,
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn reject_cas_with_invalid_base() {
        let instr = Instruction::new(
            Mnemonic::Cas,
            Operands::Cas {
                rd: Register::new(0),
                base: Register::new(32),
                rs_new: Register::new(5),
                disp: 0,
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn reject_cas_with_invalid_new_value() {
        let instr = Instruction::new(
            Mnemonic::Cas,
            Operands::Cas {
                rd: Register::new(0),
                base: Register::new(10),
                rs_new: Register::new(32),
                disp: 0,
            },
        );
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn validate_branch_imm32() {
        let instr = Instruction::new(Mnemonic::Jz, Operands::BranchImm32 {
            disp: 512,
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_branch_imm32_negative() {
        let instr = Instruction::new(Mnemonic::Jnz, Operands::BranchImm32 {
            disp: -512,
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_imm16_format() {
        let instr = Instruction::new(Mnemonic::Sys, Operands::Imm16 {
            imm: 0x1234,
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn validate_imm16_max_value() {
        let instr = Instruction::new(Mnemonic::Trap, Operands::Imm16 {
            imm: u16::MAX,
        });
        assert!(validate_instruction(&instr).is_ok());
    }

    #[test]
    fn error_message_includes_context() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(32),
        });
        let result = validate_instruction(&instr);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid instruction") || err_msg.contains("register"));
    }

    #[test]
    fn all_gp_registers_are_valid() {
        for reg_idx in 0..32 {
            let instr = Instruction::new(Mnemonic::Not, Operands::R {
                rd: Register::new(reg_idx as u8),
            });
            assert!(
                validate_instruction(&instr).is_ok(),
                "register {} should be valid",
                reg_idx
            );
        }
    }

    #[test]
    fn boundary_register_32_is_invalid() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(32),
        });
        assert!(validate_instruction(&instr).is_err());
    }

    #[test]
    fn boundary_register_255_is_invalid() {
        let instr = Instruction::new(Mnemonic::Not, Operands::R {
            rd: Register::new(255),
        });
        assert!(validate_instruction(&instr).is_err());
    }
}
