// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Static validator (spec §29): checked once, before any instruction is
//! executed, so `execution::step` never has to discover a structurally
//! malformed program mid-run.

use crate::binary;
use crate::control::branch_target;
use crate::encoding::decoder::decode_all;
use crate::errors::{BinaryFormatErrorKind, ValidationError, ValidationErrorKind};
use crate::isa::operand::Operands;

fn err(kind: ValidationErrorKind, offset: u64, reason: impl Into<String>) -> ValidationError {
    ValidationError {
        kind,
        offset,
        reason: reason.into(),
    }
}

/// Validate a complete `.jxc` file. On success, returns the decoded
/// instruction stream (offset, instruction) so callers (the CLI's
/// `validate`/`inspect`/`run` subcommands) don't have to decode twice.
pub fn validate(
    bytes: &[u8],
) -> Result<Vec<(u64, crate::isa::instruction::DecodedInstruction)>, ValidationError> {
    let program = binary::parse(bytes).map_err(|e| {
        let kind = match e.kind {
            BinaryFormatErrorKind::OffsetOutOfRange | BinaryFormatErrorKind::SizeOutOfRange => {
                ValidationErrorKind::BadSectionBounds
            }
            _ => ValidationErrorKind::BadHeader,
        };
        err(kind, 0, e.reason)
    })?;

    let instrs = decode_all(program.code).map_err(|e| {
        let kind = match e.kind {
            crate::errors::DecodeErrorKind::InvalidOpcode => ValidationErrorKind::BadOpcode,
            crate::errors::DecodeErrorKind::InvalidRegister => {
                ValidationErrorKind::BadRegisterOperand
            }
            crate::errors::DecodeErrorKind::TruncatedInstruction => {
                ValidationErrorKind::BadInstructionBoundary
            }
        };
        err(kind, e.offset, e.reason)
    })?;

    let code_len = program.code.len() as u64;
    let valid_offsets: std::collections::HashSet<u64> = instrs.iter().map(|(o, _)| *o).collect();

    for (offset, instr) in &instrs {
        if let Operands::BranchImm32 { disp } = instr.operands {
            let pc_after_fetch = offset + instr.length as u64;
            let target = branch_target(pc_after_fetch, disp);
            if target >= code_len || !valid_offsets.contains(&target) {
                return Err(err(
                    ValidationErrorKind::BadBranchTarget,
                    *offset,
                    format!(
                        "{} targets {:#x}, which is not a valid instruction boundary in the code section",
                        instr.mnemonic.text(),
                        target
                    ),
                ));
            }
        }
        // Memory operands (RMem/MemR/Cas) address `base_register + disp`;
        // the base register's runtime value is not known statically, so
        // there is nothing further to check here beyond the structural
        // decode already performed above (spec §29 "memory operands" is
        // satisfied by decode-time field validation).
    }

    if !valid_offsets.contains(&program.header.entry_point) {
        return Err(err(
            ValidationErrorKind::BadEntryPoint,
            program.header.entry_point,
            "entry point is not a valid instruction boundary in the code section",
        ));
    }

    Ok(instrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::encoder::encode_all;
    use crate::isa::instruction::DecodedInstruction;
    use crate::isa::opcodes::Mnemonic;

    #[test]
    fn accepts_well_formed_program() {
        let program = [
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 1, imm: 1 }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let code = encode_all(&program).unwrap();
        let bin = binary::write(0, &code, &[]);
        assert!(validate(&bin).is_ok());
    }

    #[test]
    fn rejects_branch_into_middle_of_another_instruction() {
        let program = [
            DecodedInstruction::new(Mnemonic::Jmp, Operands::BranchImm32 { disp: 1 }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let code = encode_all(&program).unwrap();
        let bin = binary::write(0, &code, &[]);
        let result = validate(&bin);
        assert_eq!(
            result.unwrap_err().kind,
            ValidationErrorKind::BadBranchTarget
        );
    }

    #[test]
    fn rejects_bad_entry_point() {
        let program = [DecodedInstruction::new(Mnemonic::Halt, Operands::None)];
        let code = encode_all(&program).unwrap();
        let bin = binary::write(99, &code, &[]);
        assert_eq!(
            validate(&bin).unwrap_err().kind,
            ValidationErrorKind::BadEntryPoint
        );
    }

    #[test]
    fn rejects_invalid_opcode() {
        let code = vec![0xFF]; // unassigned opcode
        let bin = binary::write(0, &code, &[]);
        assert_eq!(
            validate(&bin).unwrap_err().kind,
            ValidationErrorKind::BadOpcode
        );
    }
}
