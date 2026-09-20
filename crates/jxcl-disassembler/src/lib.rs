// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Disassembles a binary/object file back to readable assembly text, symbolizing addresses when debug info is present.
//!
//! Extracted from `jxcl/src/disassembler.rs`. Canonical form deliberately avoids inventing label names:
//! every operand that names an address (a branch target, an entry point) is printed as the *absolute
//! resolved address itself*, in decimal. Since `assembler::assemble` accepts a bare number anywhere
//! it accepts a label, and since disassembly reproduces the exact same instruction ordering and
//! therefore the exact same code-section byte layout, re-assembling this text recomputes byte-identical
//! displacements without any symbol table at all — satisfying the round-trip invariant with the
//! simplest possible canonical form.
#![forbid(unsafe_code)]

use jxcl_decoding::decode_all;
use jxcl_errors::DecodeError;
use jxcl_instructions::{Operands, Register};

fn reg(id: Register) -> String {
    format!("R{}", id.get())
}

fn mem(base: Register, disp: i32) -> String {
    match disp.cmp(&0) {
        std::cmp::Ordering::Equal => format!("[{}]", reg(base)),
        std::cmp::Ordering::Greater => format!("[{}+{}]", reg(base), disp),
        std::cmp::Ordering::Less => format!("[{}-{}]", reg(base), -(disp as i64)),
    }
}

/// Compute a PC-relative branch/call target:
/// `target = pc_after_fetch + displacement`, where `pc_after_fetch` is the
/// address of the instruction immediately following the branch.
fn branch_target(pc_after_fetch: u64, displacement: i32) -> u64 {
    (pc_after_fetch as i64).wrapping_add(displacement as i64) as u64
}

/// Render a single decoded instruction to its canonical textual form.
/// Used by the disassembler to format instructions for output.
pub fn render_instruction(
    mnemonic: jxcl_instructions::Mnemonic,
    operands: Operands,
    offset: u64,
    length: u64,
) -> String {
    let name = mnemonic.text();
    match operands {
        Operands::None => name.to_string(),
        Operands::R { rd } => format!("{} {}", name, reg(rd)),
        Operands::RR { rd, rs } => format!("{} {}, {}", name, reg(rd), reg(rs)),
        Operands::RRR { rd, rs1, rs2 } => {
            format!("{} {}, {}, {}", name, reg(rd), reg(rs1), reg(rs2))
        }
        Operands::RImm64 { rd, imm } => format!("{} {}, {}", name, reg(rd), imm),
        Operands::RMem { rd, base, disp } => format!("{} {}, {}", name, reg(rd), mem(base, disp)),
        Operands::MemR { base, disp, rs } => format!("{} {}, {}", name, mem(base, disp), reg(rs)),
        Operands::Cas {
            rd,
            base,
            rs_new,
            disp,
        } => {
            format!("{} {}, {}, {}", name, reg(rd), mem(base, disp), reg(rs_new))
        }
        Operands::BranchImm32 { disp } => {
            let pc_after_fetch = offset + length;
            let target = branch_target(pc_after_fetch, disp);
            format!("{} {}", name, target)
        }
        Operands::Imm16 { imm } => format!("{} {}", name, imm),
    }
}

/// Disassemble decoded code and data sections into canonical JXCL assembly text.
///
/// `code`: The raw code section bytes.
/// `data`: The raw data section bytes.
/// `entry_point`: The program entry point address (emitted as a `.entry` directive if non-zero).
///
/// Returns the canonical assembly text representation, or a DecodeError if the code section contains invalid instructions.
pub fn disassemble(code: &[u8], data: &[u8], entry_point: u64) -> Result<String, DecodeError> {
    let instrs = decode_all(code)?;
    let mut out = String::new();

    if entry_point != 0 {
        out.push_str(&format!(".entry {}\n", entry_point));
    }

    for (offset, instr) in &instrs {
        out.push_str(&format!(
            "{}\n",
            render_instruction(instr.mnemonic, instr.operands, *offset, instr.length as u64)
        ));
    }

    if !data.is_empty() {
        out.push_str(".data\n");
        // One byte per line keeps the canonical form width-agnostic: it
        // reproduces the exact original bytes regardless of whatever
        // mix of .byte/.word/.dword/.qword directives originally
        // produced them.
        for chunk in data.chunks(16) {
            let values: Vec<String> = chunk.iter().map(|b| b.to_string()).collect();
            out.push_str(&format!(".byte {}\n", values.join(", ")));
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_instructions::{opcode_byte, Mnemonic};

    #[test]
    fn renders_none_format_instruction() {
        let text = render_instruction(Mnemonic::Halt, Operands::None, 0, 1);
        assert_eq!(text, "HALT");
    }

    #[test]
    fn renders_r_format_instruction() {
        let text = render_instruction(
            Mnemonic::Push,
            Operands::R {
                rd: Register::new(5),
            },
            0,
            2,
        );
        assert_eq!(text, "PUSH R5");
    }

    #[test]
    fn renders_rr_format_instruction() {
        let text = render_instruction(
            Mnemonic::Add,
            Operands::RR {
                rd: Register::new(1),
                rs: Register::new(2),
            },
            0,
            3,
        );
        assert_eq!(text, "ADD R1, R2");
    }

    #[test]
    fn renders_rrr_format_instruction() {
        let text = render_instruction(
            Mnemonic::Xor3,
            Operands::RRR {
                rd: Register::new(1),
                rs1: Register::new(2),
                rs2: Register::new(3),
            },
            0,
            4,
        );
        assert_eq!(text, "XOR3 R1, R2, R3");
    }

    #[test]
    fn renders_rimm64_format_instruction() {
        let text = render_instruction(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: Register::new(1),
                imm: 42,
            },
            0,
            10,
        );
        assert_eq!(text, "MOVI R1, 42");
    }

    #[test]
    fn renders_rmem_format_instruction() {
        let text = render_instruction(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(1),
                base: Register::new(2),
                disp: 8,
            },
            0,
            7,
        );
        assert_eq!(text, "LOAD R1, [R2+8]");
    }

    #[test]
    fn renders_rmem_with_negative_displacement() {
        let text = render_instruction(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(1),
                base: Register::new(2),
                disp: -4,
            },
            0,
            7,
        );
        assert_eq!(text, "LOAD R1, [R2-4]");
    }

    #[test]
    fn renders_rmem_with_zero_displacement() {
        let text = render_instruction(
            Mnemonic::Load,
            Operands::RMem {
                rd: Register::new(1),
                base: Register::new(2),
                disp: 0,
            },
            0,
            7,
        );
        assert_eq!(text, "LOAD R1, [R2]");
    }

    #[test]
    fn renders_memr_format_instruction() {
        let text = render_instruction(
            Mnemonic::Store,
            Operands::MemR {
                base: Register::new(1),
                disp: 4,
                rs: Register::new(2),
            },
            0,
            7,
        );
        assert_eq!(text, "STORE [R1+4], R2");
    }

    #[test]
    fn renders_cas_format_instruction() {
        let text = render_instruction(
            Mnemonic::Cas,
            Operands::Cas {
                rd: Register::new(1),
                base: Register::new(2),
                rs_new: Register::new(3),
                disp: 0,
            },
            0,
            8,
        );
        assert_eq!(text, "CAS R1, [R2], R3");
    }

    #[test]
    fn renders_branch_with_positive_displacement() {
        let text = render_instruction(Mnemonic::Jmp, Operands::BranchImm32 { disp: 10 }, 100, 5);
        // pc_after_fetch = 100 + 5 = 105
        // target = 105 + 10 = 115
        assert_eq!(text, "JMP 115");
    }

    #[test]
    fn renders_branch_with_negative_displacement() {
        let text = render_instruction(Mnemonic::Jl, Operands::BranchImm32 { disp: -20 }, 100, 5);
        // pc_after_fetch = 100 + 5 = 105
        // target = 105 - 20 = 85
        assert_eq!(text, "JL 85");
    }

    #[test]
    fn renders_imm16_format_instruction() {
        let text = render_instruction(Mnemonic::Sys, Operands::Imm16 { imm: 255 }, 0, 3);
        assert_eq!(text, "SYS 255");
    }

    #[test]
    fn disassembles_empty_code_section() {
        let result = disassemble(&[], &[], 0).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn disassembles_single_instruction() {
        let code = vec![opcode_byte(Mnemonic::Halt)];
        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "HALT\n");
    }

    #[test]
    fn disassembles_multiple_instructions() {
        let code = vec![opcode_byte(Mnemonic::Nop), opcode_byte(Mnemonic::Halt)];
        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "NOP\nHALT\n");
    }

    #[test]
    fn disassembles_with_nonzero_entry_point() {
        let code = vec![opcode_byte(Mnemonic::Halt)];
        let result = disassemble(&code, &[], 256).unwrap();
        assert_eq!(result, ".entry 256\nHALT\n");
    }

    #[test]
    fn disassembles_with_data_section() {
        let code = vec![opcode_byte(Mnemonic::Halt)];
        let data = vec![1, 2, 3];
        let result = disassemble(&code, &data, 0).unwrap();
        assert_eq!(result, "HALT\n.data\n.byte 1, 2, 3\n");
    }

    #[test]
    fn disassembles_data_in_16_byte_chunks() {
        let code = vec![];
        let data: Vec<u8> = (0..20).collect();
        let result = disassemble(&code, &data, 0).unwrap();
        let expected = ".data\n.byte 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15\n.byte 16, 17, 18, 19\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn disassembles_instruction_with_register_operand() {
        let mut code = vec![opcode_byte(Mnemonic::Pop)];
        code.push(7); // Register 7
        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "POP R7\n");
    }

    #[test]
    fn disassembles_instruction_with_two_register_operands() {
        let mut code = vec![opcode_byte(Mnemonic::Sub)];
        code.push(3); // rd = R3
        code.push(4); // rs = R4
        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "SUB R3, R4\n");
    }

    #[test]
    fn disassembles_instruction_with_imm64_operand() {
        let mut code = vec![opcode_byte(Mnemonic::Movi)];
        code.push(1); // rd = R1
        code.extend_from_slice(&100u64.to_le_bytes()); // imm = 100
        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "MOVI R1, 100\n");
    }

    #[test]
    fn rejects_truncated_instruction() {
        // MOVI needs 10 bytes total; provide only 3
        let code = vec![opcode_byte(Mnemonic::Movi), 0, 0];
        let result = disassemble(&code, &[], 0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            jxcl_errors::DecodeErrorKind::TruncatedInstruction
        );
    }

    #[test]
    fn rejects_invalid_opcode() {
        let code = vec![0xFF]; // Invalid opcode
        let result = disassemble(&code, &[], 0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            jxcl_errors::DecodeErrorKind::InvalidOpcode
        );
    }

    #[test]
    fn golden_vector_simple_arithmetic() {
        // Simple program: MOVI R1, 10; ADD R1, R2; HALT
        let mut code = vec![];

        // MOVI R1, 10
        code.push(opcode_byte(Mnemonic::Movi));
        code.push(1); // rd = R1
        code.extend_from_slice(&10u64.to_le_bytes());

        // ADD R1, R2
        code.push(opcode_byte(Mnemonic::Add));
        code.push(1); // rd = R1
        code.push(2); // rs = R2

        // HALT
        code.push(opcode_byte(Mnemonic::Halt));

        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "MOVI R1, 10\nADD R1, R2\nHALT\n");
    }

    #[test]
    fn golden_vector_branching() {
        // Program: JMP +5; HALT
        let mut code = vec![];

        // JMP +5 (branch forward)
        code.push(opcode_byte(Mnemonic::Jmp));
        code.extend_from_slice(&5i32.to_le_bytes());

        // HALT at offset 5
        code.push(opcode_byte(Mnemonic::Halt));

        let result = disassemble(&code, &[], 0).unwrap();
        // pc_after_fetch for JMP is 5 (offset 0 + length 5)
        // target = 5 + 5 = 10
        assert_eq!(result, "JMP 10\nHALT\n");
    }

    #[test]
    fn golden_vector_memory_operations() {
        // Program: LOAD R1, [R2]; STORE [R3+4], R4
        let mut code = vec![];

        // LOAD R1, [R2+0]
        code.push(opcode_byte(Mnemonic::Load));
        code.push(1); // rd = R1
        code.push(2); // base = R2
        code.extend_from_slice(&0i32.to_le_bytes()); // disp = 0

        // STORE [R3+4], R4
        code.push(opcode_byte(Mnemonic::Store));
        code.push(3); // base = R3
        code.extend_from_slice(&4i32.to_le_bytes()); // disp = 4
        code.push(4); // rs = R4

        let result = disassemble(&code, &[], 0).unwrap();
        assert_eq!(result, "LOAD R1, [R2]\nSTORE [R3+4], R4\n");
    }
}
