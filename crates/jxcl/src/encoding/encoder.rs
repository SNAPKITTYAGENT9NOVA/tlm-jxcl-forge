// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Instruction encoder (spec §21): `DecodedInstruction` → bytes.
//!
//! The encoder is the strict inverse of the decoder (spec §18's round-trip
//! invariant: `encode(decode(bytes)) == bytes` for every canonical
//! encoding). It rejects anything the registry doesn't sanction — it never
//! silently truncates or reinterprets an operand.

use crate::errors::{EncodeError, EncodeErrorKind};
use crate::isa::constants::NUM_GP_REGISTERS;
use crate::isa::instruction::DecodedInstruction;
use crate::isa::opcodes::lookup_mnemonic;
use crate::isa::operand::Operands;
use crate::isa::registers::RegId;

fn check_reg(id: RegId) -> Result<(), EncodeError> {
    if (id as usize) < NUM_GP_REGISTERS {
        Ok(())
    } else {
        Err(EncodeError {
            kind: EncodeErrorKind::InvalidRegister,
            reason: format!("register id {} is outside 0..{}", id, NUM_GP_REGISTERS),
        })
    }
}

/// Encode one instruction, appending its bytes to `out`. Returns the
/// number of bytes written.
pub fn encode(instr: &DecodedInstruction, out: &mut Vec<u8>) -> Result<usize, EncodeError> {
    let def = lookup_mnemonic(instr.mnemonic);

    if instr.operands.format() != def.format {
        return Err(EncodeError {
            kind: EncodeErrorKind::InvalidOperandType,
            reason: format!(
                "{} requires format {:?}, got operands shaped as {:?}",
                instr.mnemonic.text(),
                def.format,
                instr.operands.format()
            ),
        });
    }

    let start = out.len();
    out.push(def.opcode);

    match instr.operands {
        Operands::None => {}
        Operands::R { rd } => {
            check_reg(rd)?;
            out.push(rd);
        }
        Operands::RR { rd, rs } => {
            check_reg(rd)?;
            check_reg(rs)?;
            out.push(rd);
            out.push(rs);
        }
        Operands::RRR { rd, rs1, rs2 } => {
            check_reg(rd)?;
            check_reg(rs1)?;
            check_reg(rs2)?;
            out.push(rd);
            out.push(rs1);
            out.push(rs2);
        }
        Operands::RImm64 { rd, imm } => {
            check_reg(rd)?;
            out.push(rd);
            out.extend_from_slice(&imm.to_le_bytes());
        }
        Operands::RMem { rd, base, disp } => {
            check_reg(rd)?;
            check_reg(base)?;
            out.push(rd);
            out.push(base);
            out.extend_from_slice(&disp.to_le_bytes());
        }
        Operands::MemR { base, disp, rs } => {
            check_reg(base)?;
            check_reg(rs)?;
            out.push(base);
            out.extend_from_slice(&disp.to_le_bytes());
            out.push(rs);
        }
        Operands::Cas {
            rd,
            base,
            rs_new,
            disp,
        } => {
            check_reg(rd)?;
            check_reg(base)?;
            check_reg(rs_new)?;
            out.push(rd);
            out.push(base);
            out.push(rs_new);
            out.extend_from_slice(&disp.to_le_bytes());
        }
        Operands::BranchImm32 { disp } => {
            out.extend_from_slice(&disp.to_le_bytes());
        }
        Operands::Imm16 { imm } => {
            out.extend_from_slice(&imm.to_le_bytes());
        }
    }

    let written = out.len() - start;
    debug_assert_eq!(written, def.format.len());
    Ok(written)
}

/// Encode a full instruction stream.
pub fn encode_all(instrs: &[DecodedInstruction]) -> Result<Vec<u8>, EncodeError> {
    let mut out = Vec::with_capacity(instrs.len() * 4);
    for instr in instrs {
        encode(instr, &mut out)?;
    }
    Ok(out)
}
