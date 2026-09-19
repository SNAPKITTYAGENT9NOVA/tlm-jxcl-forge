//! Minimal trace/debugger mode (spec §30): PC, instruction, register
//! changes, flag changes, and memory writes for one `step()` call,
//! rendered deterministically.

use crate::encoding::decoder::decode_one;
use crate::execution::{step, StepResult};
use crate::isa::constants::NUM_GP_REGISTERS;
use crate::machine::MachineState;

/// One instruction's worth of observable state change.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub pc: u64,
    /// Canonical textual form of the instruction that executed, or a
    /// placeholder if the byte at `pc` could not be decoded (the fault
    /// itself is still recorded in `result`).
    pub text: String,
    pub register_changes: Vec<(u8, u64, u64)>,
    pub flags_before: u64,
    pub flags_after: u64,
    pub sp_before: u64,
    pub sp_after: u64,
    /// Contiguous runs of changed memory, as `(address, old_bytes, new_bytes)`.
    pub memory_writes: Vec<(u64, Vec<u8>, Vec<u8>)>,
    pub result: StepResult,
}

fn diff_memory(before: &[u8], after: &[u8]) -> Vec<(u64, Vec<u8>, Vec<u8>)> {
    let mut writes = Vec::new();
    let mut i = 0usize;
    while i < before.len() {
        if before[i] != after[i] {
            let start = i;
            while i < before.len() && before[i] != after[i] {
                i += 1;
            }
            writes.push((start as u64, before[start..i].to_vec(), after[start..i].to_vec()));
        } else {
            i += 1;
        }
    }
    writes
}

/// Execute one instruction on `state`, returning both the `StepResult`
/// and a full trace of what architecturally changed.
pub fn trace_step(state: &mut MachineState) -> TraceEntry {
    let pc = state.registers.pc;
    let regs_before = *state.registers.general_registers();
    let sp_before = state.registers.sp;
    let flags_before = state.registers.flags;
    let mem_before = state.memory.snapshot();

    let text = match state
        .memory
        .peek(pc, 1)
        .and_then(|op| state.memory.peek(pc, crate::isa::opcodes::lookup_opcode(op[0])?.format.len() as u64))
        .and_then(|bytes| decode_one(bytes, pc).ok())
    {
        Some(instr) => {
            crate::disassembler::render_instruction(instr.mnemonic, instr.operands, pc, instr.length as u64)
        }
        None => "<undecodable>".to_string(),
    };

    let result = step(state);

    let regs_after = *state.registers.general_registers();
    let mut register_changes = Vec::new();
    for i in 0..NUM_GP_REGISTERS {
        if regs_before[i] != regs_after[i] {
            register_changes.push((i as u8, regs_before[i], regs_after[i]));
        }
    }
    let mem_after = state.memory.snapshot();

    TraceEntry {
        pc,
        text,
        register_changes,
        flags_before,
        flags_after: state.registers.flags,
        sp_before,
        sp_after: state.registers.sp,
        memory_writes: diff_memory(&mem_before, &mem_after),
        result,
    }
}

impl std::fmt::Display for TraceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:08X}  {}", self.pc, self.text)?;
        for (id, old, new) in &self.register_changes {
            writeln!(f, "  R{}: {:#x} -> {:#x}", id, old, new)?;
        }
        if self.sp_before != self.sp_after {
            writeln!(f, "  SP: {:#x} -> {:#x}", self.sp_before, self.sp_after)?;
        }
        if self.flags_before != self.flags_after {
            writeln!(f, "  FLAGS: {:#06b} -> {:#06b}", self.flags_before, self.flags_after)?;
        }
        for (addr, old, new) in &self.memory_writes {
            writeln!(f, "  MEM[{:#x}..{:#x}]: {:02x?} -> {:02x?}", addr, addr + old.len() as u64, old, new)?;
        }
        match self.result {
            StepResult::Faulted(fault) => writeln!(f, "  FAULT: {}", fault)?,
            StepResult::Signaled(sig) => writeln!(f, "  SIGNAL: {:?}", sig)?,
            StepResult::Halted => writeln!(f, "  HALTED")?,
            StepResult::Continued => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary;
    use crate::isa::instruction::DecodedInstruction;
    use crate::isa::opcodes::Mnemonic;
    use crate::isa::operand::Operands;
    use crate::memory::Memory;

    #[test]
    fn trace_reports_register_and_memory_changes() {
        let program = [
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 1, imm: 42 }),
            DecodedInstruction::new(Mnemonic::Store, Operands::MemR { base: 0, disp: 64, rs: 1 }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let code = crate::encoding::encoder::encode_all(&program).unwrap();
        let bin = binary::write(0, &code, &[]);
        let parsed = binary::parse(&bin).unwrap();
        let mem = Memory::with_code_region(256, 0, parsed.code.len() as u64).unwrap();
        let mut state = MachineState::new(mem);
        state.memory.loader_write(0, parsed.code).unwrap();

        let t1 = trace_step(&mut state);
        assert_eq!(t1.register_changes, vec![(1, 0, 42)]);

        let t2 = trace_step(&mut state);
        assert_eq!(t2.result, StepResult::Continued);
        assert_eq!(t2.memory_writes.len(), 1);
        assert_eq!(t2.memory_writes[0].0, 64);
        assert_eq!(t2.memory_writes[0].2, vec![42]); // only the low byte actually changes from 0
    }
}
