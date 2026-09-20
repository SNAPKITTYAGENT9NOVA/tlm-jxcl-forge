// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The fetch/decode/execute loop: owns instruction execution, wiring together decoding, dispatch, the ALU, memory, and exceptions.
//!
//! Owns: `step(&mut Machine)` -- the only place an instruction is actually executed.
//!
//! Implements the complete fetch/decode/execute cycle (spec §13, §20): read and validate
//! an instruction, dispatch it to the appropriate semantic handler, update architectural
//! state, and advance the program counter. Instruction semantics are delegated to the
//! split execution-group crates (`jxcl-alu`, `jxcl-control`, `jxcl-load-store`, etc.)
//! rather than hard-coded here; this module's role is orchestration and fault handling.
#![forbid(unsafe_code)]

use jxcl_decoding::decode_one;
use jxcl_errors::{DecodeErrorKind, ExecutionFault};
use jxcl_instructions::Operands;
use jxcl_machine::{Machine, Signal};
use jxcl_opcodes::{self, Mnemonic};
use jxcl_types::{Address, RegisterIndex};

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

fn fault(state: &mut Machine, f: ExecutionFault) -> StepResult {
    state.fault = Some(f);
    state.halted = true;
    StepResult::Faulted(f)
}

fn reg_read(state: &Machine, id: RegisterIndex) -> u64 {
    state.registers.read(id).unwrap_or(0)
}

fn reg_write(state: &mut Machine, id: RegisterIndex, value: u64) {
    let _ = state.registers.write(id, value);
}

/// Apply computed flags to the flags register, respecting the instruction's FlagEffect mask.
fn apply_flags(
    state: &mut Machine,
    effect: jxcl_opcodes::FlagEffect,
    computed: jxcl_alu::AluResult,
) {
    let flags = computed.flags;
    let current_flags = jxcl_flags::Flags::from_bits(state.registers.flags);
    let new_flags = jxcl_flags::Flags {
        z: if effect.z { flags.z } else { current_flags.z },
        n: if effect.n { flags.n } else { current_flags.n },
        c: if effect.c { flags.c } else { current_flags.c },
        v: if effect.v { flags.v } else { current_flags.v },
    };
    state.registers.flags = new_flags.to_bits();
}

/// Execute exactly one instruction. Idempotent once halted or faulted.
pub fn step(state: &mut Machine) -> StepResult {
    state.pending_signal = None;
    if state.halted {
        return StepResult::Halted;
    }

    let pc = state.registers.pc;

    // FETCH (opcode byte first, to learn the instruction's length).
    let opcode_byte = match state.memory.read_bytes(Address(pc), 1) {
        Ok(b) => b[0],
        Err(e) => return fault(state, ExecutionFault::from(e)),
    };
    let def = match jxcl_opcodes::lookup_opcode(opcode_byte) {
        Some(d) => d,
        None => return fault(state, ExecutionFault::InvalidOpcode),
    };
    let len = def.format.len() as u64;
    let bytes = match state.memory.read_bytes(Address(pc), len) {
        Ok(b) => b,
        Err(e) => return fault(state, ExecutionFault::from(e)),
    };

    // DECODE + VALIDATE
    let instr = match decode_one(bytes, pc) {
        Ok(i) => i,
        Err(e) => {
            let f = match e.kind {
                DecodeErrorKind::InvalidOpcode => ExecutionFault::InvalidOpcode,
                DecodeErrorKind::InvalidRegister => ExecutionFault::InvalidRegister,
                DecodeErrorKind::TruncatedInstruction => ExecutionFault::IllegalOperand,
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
            let v = try_mem!(state.memory.read64(Address(addr)));
            reg_write(state, rd, v);
        }
        (Mnemonic::Store, Operands::MemR { base, disp, rs }) => {
            let addr = addr_of(reg_read(state, base), disp);
            let v = reg_read(state, rs);
            try_mem!(state.memory.write64(Address(addr), v));
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
            try_mem!(state.memory.write64(Address(new_sp), v));
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
            let v = try_mem!(state.memory.read64(Address(sp)));
            state.registers.sp = sp + 8;
            reg_write(state, rd, v);
        }

        // ---- Two-register ALU ops ----
        (Mnemonic::Add, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::add(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Sub, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::sub(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Adc, Operands::RR { rd, rs }) => {
            let carry_in = jxcl_flags::Flags::from_bits(state.registers.flags).c;
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::adc(a, b, carry_in);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Sbc, Operands::RR { rd, rs }) => {
            let borrow_in = jxcl_flags::Flags::from_bits(state.registers.flags).c;
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::sbc(a, b, borrow_in);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Mul, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::mul(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Mulh, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::mulh(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Div, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            if b.get() == 0 {
                return fault(state, ExecutionFault::DivideByZero);
            }
            let r = jxcl_alu::div(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Rem, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            if b.get() == 0 {
                return fault(state, ExecutionFault::DivideByZero);
            }
            let r = jxcl_alu::rem(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::And, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::and(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Or, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::or(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Xor, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::xor(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Nand, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::nand(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Nor, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::nor(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Shl, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::shl(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Shr, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::shr(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Sar, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::sar(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Rol, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::rol(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Ror, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::ror(a, b);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }

        // ---- Comparison (discard result, keep flags) ----
        (Mnemonic::Cmp, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::cmp(a, b);
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Test, Operands::RR { rd, rs }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs));
            let r = jxcl_alu::test(a, b);
            apply_flags(state, def.flags, r);
        }

        // ---- One-register ALU ops ----
        (Mnemonic::Neg, Operands::R { rd }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let r = jxcl_alu::neg(a);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Inc, Operands::R { rd }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let r = jxcl_alu::inc(a);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Dec, Operands::R { rd }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let r = jxcl_alu::dec(a);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }
        (Mnemonic::Not, Operands::R { rd }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let r = jxcl_alu::not(a);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }

        // ---- Three-register logical ----
        (Mnemonic::Xor3, Operands::RRR { rd, rs1, rs2 }) => {
            let a = jxcl_types::Word::new(reg_read(state, rd));
            let b = jxcl_types::Word::new(reg_read(state, rs1));
            let c = jxcl_types::Word::new(reg_read(state, rs2));
            let r = jxcl_alu::xor3(a, b, c);
            reg_write(state, rd, r.value.get());
            apply_flags(state, def.flags, r);
        }

        // ---- Control flow ----
        (Mnemonic::Jmp, Operands::BranchImm32 { disp }) => {
            next_pc = branch_target(pc_after_fetch, disp);
        }
        (Mnemonic::Call, Operands::BranchImm32 { disp }) => {
            let sp = state.registers.sp;
            if sp < 8 {
                return fault(state, ExecutionFault::StackFault);
            }
            let new_sp = sp - 8;
            try_mem!(state.memory.write64(Address(new_sp), pc_after_fetch));
            state.registers.sp = new_sp;
            next_pc = branch_target(pc_after_fetch, disp);
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
            let target = try_mem!(state.memory.read64(Address(sp)));
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
            let flags = jxcl_flags::Flags::from_bits(state.registers.flags);
            if jxcl_control::evaluate_condition(m, flags) {
                next_pc = branch_target(pc_after_fetch, disp);
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
            let current = try_mem!(state.memory.read64(Address::new(addr)));
            let expected = reg_read(state, rd);
            let new_val = reg_read(state, rs_new);
            let (final_rd, success) = if current == expected {
                try_mem!(state.memory.write64(Address::new(addr), new_val));
                (expected, true)
            } else {
                (current, false)
            };
            reg_write(state, rd, final_rd);
            let flags = jxcl_flags::Flags {
                z: success,
                n: (final_rd as i64) < 0,
                c: false,
                v: false,
            };
            let current_flags = jxcl_flags::Flags::from_bits(state.registers.flags);
            let new_flags = jxcl_flags::Flags {
                z: if def.flags.z {
                    flags.z
                } else {
                    current_flags.z
                },
                n: if def.flags.n {
                    flags.n
                } else {
                    current_flags.n
                },
                c: if def.flags.c {
                    flags.c
                } else {
                    current_flags.c
                },
                v: if def.flags.v {
                    flags.v
                } else {
                    current_flags.v
                },
            };
            state.registers.flags = new_flags.to_bits();
        }
        (Mnemonic::Xchg, Operands::RMem { rd, base, disp }) => {
            let addr = addr_of(reg_read(state, base), disp);
            let mem_val = try_mem!(state.memory.read64(Address::new(addr)));
            let reg_val = reg_read(state, rd);
            try_mem!(state.memory.write64(Address::new(addr), reg_val));
            reg_write(state, rd, mem_val);
        }
        (Mnemonic::Fence, Operands::None) => {
            // No-op in this single-threaded reference model (spec §25)
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

fn branch_target(pc_after_fetch: u64, displacement: i32) -> u64 {
    (pc_after_fetch as i64).wrapping_add(displacement as i64) as u64
}

/// Run until halt, fault, signal, or `limit` steps have executed —
/// whichever comes first (spec §13).
pub fn run(state: &mut Machine, limit: u64) -> StepResult {
    for _ in 0..limit {
        match step(state) {
            StepResult::Continued => continue,
            other => return other,
        }
    }
    StepResult::Continued
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_instructions::DecodedInstruction;
    use jxcl_memory::Memory;

    fn make_machine(program: &[DecodedInstruction], mem_size: u64) -> Machine {
        let code = jxcl_encoding::encode_all(program).unwrap();
        let mem = Memory::new(mem_size);
        let mut state = Machine::new(mem);
        state.memory.write_bytes(Address::new(0), &code).unwrap();
        state
    }

    #[test]
    fn add_two_immediates_and_halt() {
        let program = [
            DecodedInstruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd: jxcl_registers::Register::new(1),
                    imm: 10,
                },
            ),
            DecodedInstruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd: jxcl_registers::Register::new(2),
                    imm: 20,
                },
            ),
            DecodedInstruction::new(
                Mnemonic::Add,
                Operands::RR {
                    rd: jxcl_registers::Register::new(1),
                    rs: jxcl_registers::Register::new(2),
                },
            ),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let mut state = make_machine(&program, 256);
        let result = run(&mut state, 100);
        assert_eq!(result, StepResult::Halted);
        assert_eq!(state.registers.read(RegisterIndex::new(1)).unwrap(), 30);
        assert_eq!(state.cycle_count, 4);
    }

    #[test]
    fn r0_is_hardwired_to_zero() {
        let program = [
            DecodedInstruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd: jxcl_registers::Register::new(0),
                    imm: 0xFF,
                },
            ),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let mut state = make_machine(&program, 256);
        let _ = run(&mut state, 10);
        assert_eq!(state.registers.read(RegisterIndex::new(0)).unwrap(), 0);
    }

    #[test]
    fn divide_by_zero_faults() {
        let program = [
            DecodedInstruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd: jxcl_registers::Register::new(1),
                    imm: 5,
                },
            ),
            DecodedInstruction::new(
                Mnemonic::Movi,
                Operands::RImm64 {
                    rd: jxcl_registers::Register::new(2),
                    imm: 0,
                },
            ),
            DecodedInstruction::new(
                Mnemonic::Div,
                Operands::RR {
                    rd: jxcl_registers::Register::new(1),
                    rs: jxcl_registers::Register::new(2),
                },
            ),
        ];
        let mut state = make_machine(&program, 256);
        let result = run(&mut state, 10);
        assert_eq!(result, StepResult::Faulted(ExecutionFault::DivideByZero));
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

    #[test]
    fn stack_underflow_on_ret_faults() {
        let program = [DecodedInstruction::new(Mnemonic::Ret, Operands::None)];
        let mut state = make_machine(&program, 256);
        let result = step(&mut state);
        assert_eq!(result, StepResult::Faulted(ExecutionFault::StackFault));
    }

    #[test]
    fn normal_step_continues() {
        let program = [
            DecodedInstruction::new(Mnemonic::Nop, Operands::None),
            DecodedInstruction::new(Mnemonic::Halt, Operands::None),
        ];
        let mut state = make_machine(&program, 256);
        let r = step(&mut state);
        assert_eq!(r, StepResult::Continued);
    }

    #[test]
    fn step_after_halt_returns_halted() {
        let program = [DecodedInstruction::new(Mnemonic::Halt, Operands::None)];
        let mut state = make_machine(&program, 256);
        let r1 = step(&mut state);
        assert_eq!(r1, StepResult::Halted);
        let r2 = step(&mut state);
        assert_eq!(r2, StepResult::Halted);
    }
}
