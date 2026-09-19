//! The disassembler (spec §17-§18): decoded code/data → canonical JXCL
//! assembly text.
//!
//! Canonical form deliberately avoids inventing label names: every
//! operand that names an address (a branch target, an entry point) is
//! printed as the *absolute resolved address itself*, in decimal. Since
//! `assembler::assemble` accepts a bare number anywhere it accepts a
//! label, and since disassembly reproduces the exact same instruction
//! ordering and therefore the exact same code-section byte layout,
//! re-assembling this text recomputes byte-identical displacements
//! without any symbol table at all — satisfying the round-trip invariant
//! (spec §18: `assemble(disassemble(binary)) == binary`) with the
//! simplest possible canonical form.

use crate::control::branch_target;
use crate::encoding::decoder::decode_all;
use crate::errors::DecodeError;
use crate::isa::opcodes::Mnemonic;
use crate::isa::operand::Operands;

fn reg(id: u8) -> String {
    format!("R{}", id)
}

fn mem(base: u8, disp: i32) -> String {
    match disp.cmp(&0) {
        std::cmp::Ordering::Equal => format!("[{}]", reg(base)),
        std::cmp::Ordering::Greater => format!("[{}+{}]", reg(base), disp),
        std::cmp::Ordering::Less => format!("[{}-{}]", reg(base), -(disp as i64)),
    }
}

/// Render a single decoded instruction to its canonical textual form.
/// Exposed for the debugger/trace mode (spec §30), which renders one
/// instruction at a time interleaved with register/flag/memory deltas.
pub fn render_instruction(
    mnemonic: Mnemonic,
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

/// Disassemble a decoded code section and raw data section into canonical
/// JXCL assembly text. `entry_point` is emitted as a leading `.entry`
/// directive when it is not the assembler's default of `0`.
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
        // produced them (spec §18's round-trip invariant only requires
        // byte-for-byte equality of the assembled output, not that the
        // *source* directive shapes match the original source).
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
    use crate::assembler::assemble;
    use crate::binary;

    fn roundtrip(src: &str) {
        let bin1 = assemble(src).unwrap();
        let program = binary::parse(&bin1).unwrap();
        let text = disassemble(program.code, program.data, program.header.entry_point).unwrap();
        let bin2 = assemble(&text).unwrap();
        assert_eq!(
            bin1, bin2,
            "round-trip mismatch for:\n{}\n---produced---\n{}",
            src, text
        );
    }

    #[test]
    fn roundtrips_simple_program() {
        roundtrip(
            "\
    MOVI R1, 10
    MOVI R2, 20
    ADD  R1, R2
    CMP  R1, R2
    HALT
",
        );
    }

    #[test]
    fn roundtrips_branches_and_labels() {
        roundtrip(
            "\
start:
    MOVI R1, 0
    MOVI R2, 5
loop:
    INC R1
    CMP R1, R2
    JL loop
    HALT
",
        );
    }

    #[test]
    fn roundtrips_memory_and_stack_ops() {
        roundtrip(
            "\
    MOVI R1, 8
    MOVI R2, 99
    STORE [R1], R2
    LOAD  R3, [R1]
    PUSH  R3
    POP   R4
    LEA   R5, [R1-4]
    HALT
",
        );
    }

    #[test]
    fn roundtrips_data_section() {
        roundtrip(
            "\
    MOVI R1, buffer
    HALT
.data
buffer:
    .qword 1234
    .byte 1, 2, 3
",
        );
    }

    #[test]
    fn roundtrips_nonzero_entry_point() {
        roundtrip(
            "\
.entry mid
    HALT
mid:
    HALT
",
        );
    }

    #[test]
    fn roundtrips_call_ret_and_atomics() {
        roundtrip(
            "\
    MOVI R1, 0
    CALL fn
    HALT
fn:
    MOVI R2, 1
    XCHG R2, [R1]
    CAS  R2, [R1], R3
    FENCE
    RET
",
        );
    }
}
