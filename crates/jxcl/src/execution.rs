// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The execution engine (spec §13): fetch → decode → validate → execute →
//! update state → next PC.
//!
//! Every architecturally defined outcome — a normal step, HALT, a SYS/TRAP
//! signal, or a fault — is represented as an ordinary `StepResult` value.
//! Nothing here relies on a host-language panic to define ISA behavior
//! (spec §11, §42).

use crate::alu;
use crate::control;
use crate::errors::ExecutionFault;
use crate::isa::flags::{apply_effect, Flags};
use crate::isa::opcodes::{lookup_opcode, Mnemonic};
use crate::isa::operand::Operands;
use crate::isa::registers::RegId;
use crate::machine::{MachineState, Signal};

/// Outcome of a single `step()` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// The machine executed one instruction and is ready to continue.
    Continued,
    /// The machine executed HALT; further `step()` calls are no-ops that
    /// keep returning `Halted`.
    Halted,
    /// The machine executed SYS/TRAP; `state.pending_signal` names it.
    Signaled(Signal),
    /// Execution stopped on an architectural fault; `state.fault` names it.
    Faulted(ExecutionFault),
}

fn addr_of(base_value: u64, disp: i32) -> u64 {
    (base_value as i64).wrapping_add(disp as i64) as u64
}

fn fault(state: &mut MachineState, f: ExecutionFault) -> StepResult {
    state.fault = Some(f);
    state.halted = true;
    StepResult::Faulted(f)
}

fn reg_read(state: &MachineState, id: RegId) -> u64 {
    // Safety of `.expect`: every RegId reaching here was already
    // range-checked by the decoder (spec §20 step 5), so this can never
    // fail in practice; it is not a silent fallback, it is a defensive
    // re-assertion of a decode-time invariant.
    state
        .registers
        .read(id)
        .expect("decoder already validated register operands")
}

fn reg_write(state: &mut MachineState, id: RegId, value: u64) {
    state
        .registers
        .write(id, value)
        .expect("decoder already validated register operands");
}

/// Execute exactly one instruction. Idempotent once halted or faulted.
pub fn step(state: &mut MachineState) -> StepResult {
    state.pending_signal = None;
    if state.halted {
        return StepResult::Halted;
    }

    let pc = state.registers.pc;

    // FETCH (opcode byte first, to learn the instruction's length).
    let opcode_byte = match state.memory.fetch(pc, 1) {
        Ok(b) => b[0],
        Err(e) => return fault(state, ExecutionFault::from(e)),
    };
    let def = match lookup_opcode(opcode_byte) {
        Some(d) => d,
        None => return fault(state, ExecutionFault::InvalidOpcode),
    };
    let len = def.format.len() as u64;
    let bytes = match state.memory.fetch(pc, len) {
        Ok(b) => b,
        Err(e) => return fault(state, ExecutionFault::from(e)),
    };

    // DECODE + VALIDATE (register-range validation happens inside decode_one).
    let instr = match crate::encoding::decoder::decode_one(bytes, pc) {
        Ok(i) => i,
        Err(e) => {
            let f = match e.kind {
                crate::errors::DecodeErrorKind::InvalidOpcode => ExecutionFault::InvalidOpcode,
                crate::errors::DecodeErrorKind::InvalidRegister => ExecutionFault::InvalidRegister,
                crate::errors::DecodeErrorKind::TruncatedInstruction => {
                    ExecutionFault::IllegalOperand
                }
            };
            return fault(state, f);
        }
    };

    let pc_after_fetch = pc + len;
    let mut next_pc = pc_after_fetch;
    let mut signal: Option<Signal> = None;

    macro_rules! try_mem {
        ($expr:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => return fault(state, ExecutionFault::from(e)),
            }
        };
    }

    match (instr.mnemonic, instr.operands) {
        (Mnemonic::Nop, Operands::None) => {}

        // ---- Data movement ----
        (Mnemonic::Mov, Operands::RR { rd, rs }) => {
            let v = reg_read(state, rs);
            reg_write(state, rd, v);
        }
        (Mnemonic::Movi, Operands::RImm64 { rd, imm }) => {
            reg_write(state, rd, imm);
        }
        (Mnemonic::Load, Operands::RMem { rd, base, disp }) => {
            let addr = addr_of(reg_read(state, base), disp);
            let v = try_mem!(state.memory.read64(addr));
            reg_write(state, rd, v);
        }
        (Mnemonic::Store, Operands::MemR { base, disp, rs }) => {
            let addr = addr_of(reg_read(state, base), disp);
            let v = reg_read(state, rs);
            try_mem!(state.memory.write64(addr, v));
        }
        (Mnemonic::Lea, Operands::RMem { rd, base, disp }) => {
            let addr = addr_of(reg_read(state, base), disp);
            reg_write(state, rd, addr);
        }
        (Mnemonic::Push, Operands::R { rd }) => {
            let sp = state.registers.sp;
            if sp < 8 {
                return fault(state, ExecutionFault::StackFault);
            }
            let new_sp = sp - 8;
            let v = reg_read(state, rd);
            try_mem!(state.memory.write64(new_sp, v));
            state.registers.sp = new_sp;
        }
        (Mnemonic::Pop, Operands::R { rd }) => {
            let sp = state.registers.sp;
            if sp
                .checked_add(8)
                .map(|e| e > state.memory.len())
                .unwrap_or(true)
            {
                return fault(state, ExecutionFault::StackFault);
            }
            let v = try_mem!(state.memory.read64(sp));
            state.registers.sp = sp + 8;
            reg_write(state, rd, v);
        }

        // ---- Two-register ALU ops ----
        (Mnemonic::Add, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::add),
        (Mnemonic::Sub, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::sub),
        (Mnemonic::Adc, Operands::RR { rd, rs }) => {
            let carry_in = Flags::from_bits(state.registers.flags).c;
            let a = reg_read(state, rd);
            let b = reg_read(state, rs);
            let r = alu::adc(a, b, carry_in);
            reg_write(state, rd, r.value);
            apply_flags(state, def, r.flags);
        }
        (Mnemonic::Sbc, Operands::RR { rd, rs }) => {
            let borrow_in = Flags::from_bits(state.registers.flags).c;
            let a = reg_read(state, rd);
            let b = reg_read(state, rs);
            let r = alu::sbc(a, b, borrow_in);
            reg_write(state, rd, r.value);
            apply_flags(state, def, r.flags);
        }
        (Mnemonic::Mul, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::mul),
        (Mnemonic::Mulh, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::mulh),
        (Mnemonic::Div, Operands::RR { rd, rs }) => {
            let a = reg_read(state, rd);
            let b = reg_read(state, rs);
            if b == 0 {
                return fault(state, ExecutionFault::DivideByZero);
            }
            let r = alu::div(a, b);
            reg_write(state, rd, r.value);
            apply_flags(state, def, r.flags);
        }
        (Mnemonic::Rem, Operands::RR { rd, rs }) => {
            let a = reg_read(state, rd);
            let b = reg_read(state, rs);
            if b == 0 {
                return fault(state, ExecutionFault::DivideByZero);
            }
            let r = alu::rem(a, b);
            reg_write(state, rd, r.value);
            apply_flags(state, def, r.flags);
        }
        (Mnemonic::And, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::and),
        (Mnemonic::Or, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::or),
        (Mnemonic::Xor, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::xor),
        (Mnemonic::Nand, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::nand),
        (Mnemonic::Nor, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::nor),
        (Mnemonic::Shl, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::shl),
        (Mnemonic::Shr, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::shr),
        (Mnemonic::Sar, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::sar),
        (Mnemonic::Rol, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::rol),
        (Mnemonic::Ror, Operands::RR { rd, rs }) => bin_alu(state, def, rd, rs, alu::ror),

        // ---- Comparison (discard result, keep flags) ----
        (Mnemonic::Cmp, Operands::RR { rd, rs }) => {
            let a = reg_read(state, rd);
            let b = reg_read(state, rs);
            let r = alu::cmp(a, b);
            apply_flags(state, def, r.flags);
        }
        (Mnemonic::Test, Operands::RR { rd, rs }) => {
            let a = reg_read(state, rd);
            let b = reg_read(state, rs);
            let r = alu::test(a, b);
            apply_flags(state, def, r.flags);
        }

        // ---- One-register ALU ops ----
        (Mnemonic::Neg, Operands::R { rd }) => un_alu(state, def, rd, alu::neg),
        (Mnemonic::Inc, Operands::R { rd }) => un_alu(state, def, rd, alu::inc),
        (Mnemonic::Dec, Operands::R { rd }) => un_alu(state, def, rd, alu::dec),
        (Mnemonic::Not, Operands::R { rd }) => un_alu(state, def, rd, alu::not),

        // ---- Three-register logical ----
        (Mnemonic::Xor3, Operands::RRR { rd, rs1, rs2 }) => {
            let a = reg_read(state, rd);
            let b = reg_read(state, rs1);
            let c = reg_read(state, rs2);
            let r = alu::xor3(a, b, c);
            reg_write(state, rd, r.value);
            apply_flags(state, def, r.flags);
        }

        // ---- Control flow ----
        (Mnemonic::Jmp, Operands::BranchImm32 { disp }) => {
            next_pc = control::branch_target(pc_after_fetch, disp);
        }
        (Mnemonic::Call, Operands::BranchImm32 { disp }) => {
            let sp = state.registers.sp;
            if sp < 8 {
                return fault(state, ExecutionFault::StackFault);
            }
            let new_sp = sp - 8;
            try_mem!(state.memory.write64(new_sp, pc_after_fetch));
            state.registers.sp = new_sp;
            next_pc = control::branch_target(pc_after_fetch, disp);
        }
        (Mnemonic::Ret, Operands::None) => {
            let sp = state.registers.sp;
            if sp
                .checked_add(8)
                .map(|e| e > state.memory.len())
                .unwrap_or(true)
            {
                return fault(state, ExecutionFault::StackFault);
            }
            let target = try_mem!(state.memory.read64(sp));
            state.registers.sp = sp + 8;
            next_pc = target;
        }
        (
            m @ (Mnemonic::Jz
            | Mnemonic::Jnz
            | Mnemonic::Jc
            | Mnemonic::Jnc
            | Mnemonic::Jl
            | Mnemonic::Jle
            | Mnemonic::Jg
            | Mnemonic::Jge),
            Operands::BranchImm32 { disp },
        ) => {
            let flags = Flags::from_bits(state.registers.flags);
            if control::condition_holds(m, flags) {
                next_pc = control::branch_target(pc_after_fetch, disp);
            }
        }

        // ---- System ----
        (Mnemonic::Halt, Operands::None) => {
            state.halted = true;
        }
        (Mnemonic::Trap, Operands::Imm16 { imm }) => {
            signal = Some(Signal::Trap(imm));
        }
        (Mnemonic::Sys, Operands::Imm16 { imm }) => {
            signal = Some(Signal::Syscall(imm));
        }

        // ---- Memory / atomic ----
        (
            Mnemonic::Cas,
            Operands::Cas {
                rd,
                base,
                rs_new,
                disp,
            },
        ) => {
            let addr = addr_of(reg_read(state, base), disp);
            let current = try_mem!(state.memory.read64(addr));
            let expected = reg_read(state, rd);
            let new_val = reg_read(state, rs_new);
            let (final_rd, success) = if current == expected {
                try_mem!(state.memory.write64(addr, new_val));
                (expected, true)
            } else {
                (current, false)
            };
            reg_write(state, rd, final_rd);
            let flags = Flags {
                z: success,
                n: (final_rd as i64) < 0,
                c: false,
                v: false,
            };
            apply_flags(state, def, flags);
        }
        (Mnemonic::Xchg, Operands::RMem { rd, base, disp }) => {
            let addr = addr_of(reg_read(state, base), disp);
            let mem_val = try_mem!(state.memory.read64(addr));
            let reg_val = reg_read(state, rd);
            try_mem!(state.memory.write64(addr, reg_val));
            reg_write(state, rd, mem_val);
        }
        (Mnemonic::Fence, Operands::None) => {
            // No-op in this single-threaded reference model (spec §25):
            // there is no concurrent agent whose visibility order a
            // fence could affect. Kept as a real, decodable instruction
            // so instruction streams designed for a concurrent JXCL
            // backend still assemble/execute here unchanged.
        }

        (m, ops) => {
            unreachable!(
                "registry/format mismatch: {:?} decoded with operand shape {:?}",
                m,
                ops.format()
            );
        }
    }

    state.registers.pc = next_pc;
    state.cycle_count += 1;
    state.pending_signal = signal;

    match signal {
        Some(s) => StepResult::Signaled(s),
        None if state.halted => StepResult::Halted,
        None => StepResult::Continued,
    }
}

fn bin_alu(
    state: &mut MachineState,
    def: &crate::isa::opcodes::InstructionDef,
    rd: RegId,
    rs: RegId,
    op: fn(u64, u64) -> alu::AluResult,
) {
    let a = reg_read(state, rd);
    let b = reg_read(state, rs);
    let r = op(a, b);
    reg_write(state, rd, r.value);
    apply_flags(state, def, r.flags);
}

fn un_alu(
    state: &mut MachineState,
    def: &crate::isa::opcodes::InstructionDef,
    rd: RegId,
    op: fn(u64) -> alu::AluResult,
) {
    let a = reg_read(state, rd);
    let r = op(a);
    reg_write(state, rd, r.value);
    apply_flags(state, def, r.flags);
}

fn apply_flags(
    state: &mut MachineState,
    def: &crate::isa::opcodes::InstructionDef,
    computed: Flags,
) {
    state.registers.flags = apply_effect(state.registers.flags, def.flags, computed);
}

/// Run until halt, fault, signal, or `limit` steps have executed —
/// whichever comes first (spec §13: "an optional execution limit to
/// prevent infinite programs from running indefinitely").
pub fn run(state: &mut MachineState, limit: u64) -> StepResult {
    for _ in 0..limit {
        match step(state) {
            StepResult::Continued => continue,
            other => return other,
        }
    }
    StepResult::Continued
}

/// Run until halt, fault, or signal, with the default execution limit
/// (spec §13 `run()`).
pub fn run_default(state: &mut MachineState) -> StepResult {
    run(state, crate::isa::constants::DEFAULT_EXECUTION_LIMIT)
}

/// Run until the machine halts or faults, ignoring signals (spec §13
/// `run_until_halt()`). Returns the terminal `StepResult`.
pub fn run_until_halt(state: &mut MachineState, limit: u64) -> StepResult {
    for _ in 0..limit {
        match step(state) {
            StepResult::Continued | StepResult::Signaled(_) => continue,
            other => return other,
        }
    }
    StepResult::Continued
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::encoder::encode_all;
    use crate::isa::instruction::DecodedInstruction;
    use crate::memory::Memory;

    fn make_machine(program: &[DecodedInstruction], mem_size: u64) -> MachineState {
        let code = encode_all(program).unwrap();
        let mem = Memory::with_code_region(mem_size, 0, code.len() as u64).unwrap();
        let mut state = MachineState::new(mem);
        state.memory.loader_write(0, &code).unwrap();
        state
    }

    #[test]
    fn add_two_immediates_and_halt() {
        let program = [
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 1, imm: 10 }),
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 2, imm: 20 }),
            DecodedInstruction::new(Mnemonic::Add, Operands::RR { rd: 1, rs: 2 }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let mut state = make_machine(&program, 256);
        let result = run_until_halt(&mut state, 100);
        assert_eq!(result, StepResult::Halted);
        assert_eq!(state.registers.read(1).unwrap(), 30);
        assert_eq!(state.cycle_count, 4);
    }

    #[test]
    fn r0_is_hardwired_to_zero() {
        let program = [
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 0, imm: 0xFF }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let mut state = make_machine(&program, 256);
        run_until_halt(&mut state, 10);
        assert_eq!(state.registers.read(0).unwrap(), 0);
    }

    #[test]
    fn divide_by_zero_faults() {
        let program = [
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 1, imm: 5 }),
            DecodedInstruction::new(Mnemonic::Movi, Operands::RImm64 { rd: 2, imm: 0 }),
            DecodedInstruction::new(Mnemonic::Div, Operands::RR { rd: 1, rs: 2 }),
        ];
        let mut state = make_machine(&program, 256);
        let result = run_until_halt(&mut state, 10);
        assert_eq!(result, StepResult::Faulted(ExecutionFault::DivideByZero));
    }

    #[test]
    fn call_and_ret_roundtrip_pc() {
        // Layout: [0] CALL disp  [5] HALT  [6] RET
        // pc_after_fetch for CALL is 5 (0 + len(CALL)); RET sits at offset
        // 6, i.e. one byte (len(HALT)) past pc_after_fetch.
        let halt_len = crate::isa::operand::Format::None.len() as i32;
        let program = [
            DecodedInstruction::new(Mnemonic::Call, Operands::BranchImm32 { disp: halt_len }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
            DecodedInstruction::new(Mnemonic::Ret, Operands::None),
        ];
        let mem = {
            let code = encode_all(&program).unwrap();
            let mem = Memory::with_code_region(256, 0, code.len() as u64).unwrap();
            let mut m = mem;
            m.loader_write(0, &code).unwrap();
            m
        };
        let mut state = MachineState::new(mem);
        // step 1: CALL -> jumps to RET, pushing return addr (offset 5, the HALT).
        let r1 = step(&mut state);
        assert_eq!(r1, StepResult::Continued);
        assert_eq!(state.registers.pc, 6);
        // step 2: RET -> pops back to the HALT at offset 5.
        let r2 = step(&mut state);
        assert_eq!(r2, StepResult::Continued);
        assert_eq!(state.registers.pc, 5);
        // step 3: HALT.
        let r3 = step(&mut state);
        assert_eq!(r3, StepResult::Halted);
    }

    #[test]
    fn stack_underflow_on_ret_without_call_faults() {
        let program = [DecodedInstruction::new(Mnemonic::Ret, Operands::None)];
        let mut state = make_machine(&program, 256);
        let result = step(&mut state);
        assert_eq!(result, StepResult::Faulted(ExecutionFault::StackFault));
    }

    #[test]
    fn trap_signals_without_halting() {
        let program = [
            DecodedInstruction::new(Mnemonic::Trap, Operands::Imm16 { imm: 42 }),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let mut state = make_machine(&program, 256);
        let r1 = step(&mut state);
        assert_eq!(r1, StepResult::Signaled(Signal::Trap(42)));
        assert_eq!(state.pending_signal, Some(Signal::Trap(42)));
        let r2 = step(&mut state);
        assert_eq!(r2, StepResult::Halted);
    }
}
