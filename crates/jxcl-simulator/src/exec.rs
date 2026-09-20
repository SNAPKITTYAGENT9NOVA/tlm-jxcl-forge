// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The fetch/decode/execute loop for this crate's embedded ISA subset,
//! ported from `crates/jxcl/src/execution.rs` and `src/control.rs`.

use crate::alu::{self, Flags};
use crate::decode::{decode_one, DecodeErrorKind, Operands};
use crate::isa::Mnemonic;
use crate::state::{ExecutionFault, MachineState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Continued,
    Halted,
    Faulted(ExecutionFault),
}

fn addr_of(base_value: u64, disp: i32) -> u64 {
    (base_value as i64).wrapping_add(disp as i64) as u64
}

fn branch_target(pc_after_fetch: u64, displacement: i32) -> u64 {
    (pc_after_fetch as i64).wrapping_add(displacement as i64) as u64
}

fn condition_holds(mnemonic: Mnemonic, flags: Flags) -> bool {
    match mnemonic {
        Mnemonic::Jz => flags.z,
        Mnemonic::Jnz => !flags.z,
        Mnemonic::Jl => flags.n != flags.v,
        Mnemonic::Jge => flags.n == flags.v,
        Mnemonic::Jg => !flags.z && (flags.n == flags.v),
        Mnemonic::Jle => flags.z || (flags.n != flags.v),
        other => unreachable!("condition_holds called with non-conditional mnemonic {other:?}"),
    }
}

fn fault(state: &mut MachineState, f: ExecutionFault) -> StepResult {
    state.fault = Some(f);
    state.halted = true;
    StepResult::Faulted(f)
}

macro_rules! try_mem {
    ($state:expr, $expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return fault($state, ExecutionFault::from(e)),
        }
    };
}

macro_rules! try_reg {
    ($state:expr, $expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return fault($state, e),
        }
    };
}

/// Execute exactly one instruction. Idempotent once halted or faulted.
pub fn step(state: &mut MachineState) -> StepResult {
    if state.halted {
        return StepResult::Halted;
    }

    let pc = state.pc;
    let opcode_byte = match state.memory.fetch(pc, 1) {
        Ok(b) => b[0],
        Err(e) => return fault(state, ExecutionFault::from(e)),
    };
    let def = match crate::isa::lookup_opcode(opcode_byte) {
        Some(d) => d,
        None => return fault(state, ExecutionFault::InvalidOpcode),
    };
    let len = def.format.len() as u64;
    let bytes = match state.memory.fetch(pc, len) {
        Ok(b) => b,
        Err(e) => return fault(state, ExecutionFault::from(e)),
    };

    let instr = match decode_one(bytes, pc) {
        Ok(i) => i,
        Err(e) => {
            let f = match e.kind {
                DecodeErrorKind::InvalidOpcode => ExecutionFault::InvalidOpcode,
                DecodeErrorKind::InvalidRegister => ExecutionFault::InvalidRegister,
                DecodeErrorKind::TruncatedInstruction => ExecutionFault::TruncatedInstruction,
            };
            return fault(state, f);
        }
    };

    let pc_after_fetch = pc + len;
    let mut next_pc = pc_after_fetch;

    macro_rules! reg_read {
        ($id:expr) => {
            try_reg!(state, state.read_reg($id))
        };
    }
    macro_rules! reg_write {
        ($id:expr, $val:expr) => {{
            let v = $val;
            try_reg!(state, state.write_reg($id, v));
        }};
    }
    macro_rules! bin_alu {
        ($rd:expr, $rs:expr, $op:expr) => {{
            let a = reg_read!($rd);
            let b = reg_read!($rs);
            let r = $op(a, b);
            reg_write!($rd, r.value);
            state.flags = r.flags.to_bits();
        }};
    }

    match (instr.mnemonic, instr.operands) {
        (Mnemonic::Nop, Operands::None) => {}
        (Mnemonic::Mov, Operands::RR { rd, rs }) => {
            let v = reg_read!(rs);
            reg_write!(rd, v);
        }
        (Mnemonic::Movi, Operands::RImm64 { rd, imm }) => reg_write!(rd, imm),
        (Mnemonic::Load, Operands::RMem { rd, base, disp }) => {
            let addr = addr_of(reg_read!(base), disp);
            let v = try_mem!(state, state.memory.read64(addr));
            reg_write!(rd, v);
        }
        (Mnemonic::Store, Operands::MemR { base, disp, rs }) => {
            let addr = addr_of(reg_read!(base), disp);
            let v = reg_read!(rs);
            try_mem!(state, state.memory.write64(addr, v));
        }
        (Mnemonic::Push, Operands::R { rd }) => {
            let sp = state.sp;
            if sp < 8 {
                return fault(state, ExecutionFault::StackFault);
            }
            let new_sp = sp - 8;
            let v = reg_read!(rd);
            try_mem!(state, state.memory.write64(new_sp, v));
            state.sp = new_sp;
        }
        (Mnemonic::Pop, Operands::R { rd }) => {
            let sp = state.sp;
            if sp
                .checked_add(8)
                .map(|e| e > state.memory.len())
                .unwrap_or(true)
            {
                return fault(state, ExecutionFault::StackFault);
            }
            let v = try_mem!(state, state.memory.read64(sp));
            state.sp = sp + 8;
            reg_write!(rd, v);
        }
        (Mnemonic::Add, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::add),
        (Mnemonic::Sub, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::sub),
        (Mnemonic::Mul, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::mul),
        (Mnemonic::Div, Operands::RR { rd, rs }) => {
            let a = reg_read!(rd);
            let b = reg_read!(rs);
            if b == 0 {
                return fault(state, ExecutionFault::DivideByZero);
            }
            let r = alu::div(a, b);
            reg_write!(rd, r.value);
            state.flags = r.flags.to_bits();
        }
        (Mnemonic::And, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::and),
        (Mnemonic::Or, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::or),
        (Mnemonic::Xor, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::xor),
        (Mnemonic::Shl, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::shl),
        (Mnemonic::Shr, Operands::RR { rd, rs }) => bin_alu!(rd, rs, alu::shr),
        (Mnemonic::Cmp, Operands::RR { rd, rs }) => {
            let a = reg_read!(rd);
            let b = reg_read!(rs);
            let r = alu::cmp(a, b);
            state.flags = r.flags.to_bits();
        }
        (Mnemonic::Inc, Operands::R { rd }) => {
            let a = reg_read!(rd);
            let r = alu::inc(a);
            reg_write!(rd, r.value);
            state.flags = r.flags.to_bits();
        }
        (Mnemonic::Dec, Operands::R { rd }) => {
            let a = reg_read!(rd);
            let r = alu::dec(a);
            reg_write!(rd, r.value);
            state.flags = r.flags.to_bits();
        }
        (Mnemonic::Not, Operands::R { rd }) => {
            let a = reg_read!(rd);
            let r = alu::not(a);
            reg_write!(rd, r.value);
            state.flags = r.flags.to_bits();
        }
        (Mnemonic::Jmp, Operands::BranchImm32 { disp }) => {
            next_pc = branch_target(pc_after_fetch, disp);
        }
        (Mnemonic::Call, Operands::BranchImm32 { disp }) => {
            let sp = state.sp;
            if sp < 8 {
                return fault(state, ExecutionFault::StackFault);
            }
            let new_sp = sp - 8;
            try_mem!(state, state.memory.write64(new_sp, pc_after_fetch));
            state.sp = new_sp;
            next_pc = branch_target(pc_after_fetch, disp);
        }
        (Mnemonic::Ret, Operands::None) => {
            let sp = state.sp;
            if sp
                .checked_add(8)
                .map(|e| e > state.memory.len())
                .unwrap_or(true)
            {
                return fault(state, ExecutionFault::StackFault);
            }
            let target = try_mem!(state, state.memory.read64(sp));
            state.sp = sp + 8;
            next_pc = target;
        }
        (
            m @ (Mnemonic::Jz
            | Mnemonic::Jnz
            | Mnemonic::Jl
            | Mnemonic::Jle
            | Mnemonic::Jg
            | Mnemonic::Jge),
            Operands::BranchImm32 { disp },
        ) => {
            let flags = Flags::from_bits(state.flags);
            if condition_holds(m, flags) {
                next_pc = branch_target(pc_after_fetch, disp);
            }
        }
        (Mnemonic::Halt, Operands::None) => {
            state.halted = true;
        }
        (m, ops) => {
            unreachable!("registry/format mismatch: {:?} with operands {:?}", m, ops);
        }
    }

    state.pc = next_pc;
    state.cycle_count += 1;

    if state.halted {
        StepResult::Halted
    } else {
        StepResult::Continued
    }
}

/// Run until halt, fault, or `limit` steps have executed.
pub fn run(state: &mut MachineState, limit: u64) -> StepResult {
    let mut last = StepResult::Continued;
    for _ in 0..limit {
        last = step(state);
        if !matches!(last, StepResult::Continued) {
            return last;
        }
    }
    last
}
