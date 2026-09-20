// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! An embeddable top-level simulation API (load + run in one call) for
//! using jxcl as a library from other Rust programs/services.
//!
//! Owns: `Simulator::new`/`run`.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` lists `jxcl-loader`, `jxcl-machine`, `jxcl-execution`
//! and `jxcl-memory-map` as this crate's workspace dependencies (`Cargo.toml`
//! declares all four as path dependencies to match). As of this batch all
//! four are still scaffolded placeholders with no public API to compose --
//! there is no `Machine`, `step`, `load`, or `MemoryMap` to call yet. Rather
//! than block on that (per this batch's instructions), `Simulator` embeds a
//! real, working fetch/decode/execute loop of its own (`isa`, `decode`,
//! `alu`, `state`, `exec`), ported line-for-line where feasible from
//! `crates/jxcl`'s ground-truth `src/isa/opcodes.rs`, `src/encoding/decoder.rs`,
//! `src/alu.rs`, `src/machine.rs`, `src/memory.rs`, `src/control.rs` and
//! `src/execution.rs` -- with two deliberate scope reductions, documented
//! where they're made:
//!
//! 1. **Opcode coverage**: a real subset of 30 of the full registry's 47
//!    mnemonics (data movement, integer arithmetic, logic, shifts,
//!    comparison, unconditional/signed-conditional control flow, HALT),
//!    every one of them at its real opcode byte and wire format from
//!    `crates/jxcl`, so hand-encoded bytes for these mnemonics are valid
//!    real JXCL bytecode. Opcodes outside this subset (ADC/SBC/MULH/REM/
//!    NEG/logic NAND/NOR/XOR3/SAR/ROL/ROR/TEST/unsigned Jc/Jnc/TRAP/SYS/
//!    CAS/XCHG/FENCE) decode as `InvalidOpcode` here; they are exactly
//!    `jxcl-execution`'s job to add once it lands, at which point
//!    `Simulator` can delegate to it and drop this embedded loop.
//! 2. **Memory model**: a flat, bounded, little-endian memory with bounds
//!    and alignment checks but *no* code/data permission split --
//!    that split is `jxcl-memory-map`'s and `jxcl-memory`'s job, not
//!    reinvented here.
//!
//! Once the four real dependency crates land, `Simulator::new`/`run`'s
//! public signature does not need to change: its body becomes
//! `jxcl_loader::load` + a loop over `jxcl_execution::step` on a real
//! `jxcl_machine::Machine`, laid out per `jxcl_memory_map::DEFAULT_MEMORY_MAP`.
#![forbid(unsafe_code)]

mod alu;
mod decode;
mod exec;
mod isa;
mod state;

use isa::NUM_GP_REGISTERS;
use state::{MachineState, Memory};

/// Extra memory (beyond the program image) reserved for the stack, for
/// programs constructed with [`Simulator::new`]'s default sizing.
const DEFAULT_STACK_RESERVE: u64 = 4096;
const DEFAULT_EXECUTION_LIMIT: u64 = isa::DEFAULT_EXECUTION_LIMIT;

/// The final architectural state after a run, in a form independent of
/// this crate's internal `MachineState` representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub halted: bool,
    pub fault: Option<String>,
    pub cycles: u64,
    pub pc: u64,
    pub sp: u64,
    pub flags: u64,
    pub registers: [u64; NUM_GP_REGISTERS],
}

impl RunResult {
    /// Convenience accessor for one general-purpose register's final
    /// value (`0` for an out-of-range id, matching R0's hardwired-zero
    /// convention rather than panicking).
    pub fn register(&self, id: u8) -> u64 {
        self.registers.get(id as usize).copied().unwrap_or(0)
    }
}

/// An embeddable JXCL machine: load a raw instruction-byte image and run
/// it to completion in one call.
#[derive(Debug)]
pub struct Simulator {
    state: MachineState,
}

impl Simulator {
    /// Load `binary_bytes` as the initial code image at address 0 and
    /// construct a machine ready to run: PC = 0, SP = end of memory
    /// (matching the downward-growing-stack convention), a working
    /// memory of `binary_bytes.len() + a fixed stack reserve` bytes.
    ///
    /// `binary_bytes` is raw hand-encoded (or, once `jxcl-assembler`
    /// lands, assembler-produced) instruction bytes -- there is no
    /// `jxcl-loader` container header to parse yet (see this module's
    /// doc comment).
    pub fn new(binary_bytes: impl Into<Vec<u8>>) -> Self {
        let code = binary_bytes.into();
        let mem_size = code.len() as u64 + DEFAULT_STACK_RESERVE;
        Self::with_memory_size(code, mem_size).expect("default sizing always fits the code image")
    }

    /// Like [`Simulator::new`], but with an explicit total memory size.
    /// Returns `Err` if `memory_size` is smaller than `binary_bytes`.
    pub fn with_memory_size(
        binary_bytes: impl Into<Vec<u8>>,
        memory_size: u64,
    ) -> Result<Self, String> {
        let code = binary_bytes.into();
        if memory_size < code.len() as u64 {
            return Err(format!(
                "memory_size {memory_size} is smaller than the {}-byte program image",
                code.len()
            ));
        }
        let mut memory = Memory::new(memory_size);
        memory
            .load(0, &code)
            .map_err(|e| format!("failed to load program image: {e}"))?;
        let state = MachineState {
            registers: [0; NUM_GP_REGISTERS],
            pc: 0,
            sp: memory_size,
            flags: 0,
            memory,
            halted: false,
            fault: None,
            cycle_count: 0,
        };
        Ok(Simulator { state })
    }

    /// Run to completion (halt or fault), or until the default execution
    /// limit is reached, whichever comes first.
    pub fn run(&mut self) -> RunResult {
        self.run_with_limit(DEFAULT_EXECUTION_LIMIT)
    }

    /// Run to completion (halt or fault), or until `limit` instructions
    /// have executed, whichever comes first.
    pub fn run_with_limit(&mut self, limit: u64) -> RunResult {
        exec::run(&mut self.state, limit);
        self.result()
    }

    /// Execute exactly one instruction (a no-op if already halted) and
    /// report the resulting state. Exposed for callers (e.g. a future
    /// `jxcl-debugger` wrapper) that want to single-step an embedded
    /// simulation rather than run it to completion in one call.
    pub fn step(&mut self) -> RunResult {
        exec::step(&mut self.state);
        self.result()
    }

    pub fn is_halted(&self) -> bool {
        self.state.halted
    }

    pub fn pc(&self) -> u64 {
        self.state.pc
    }

    fn result(&self) -> RunResult {
        RunResult {
            halted: self.state.halted,
            fault: self.state.fault.map(|f| format!("{f:?}")),
            cycles: self.state.cycle_count,
            pc: self.state.pc,
            sp: self.state.sp,
            flags: self.state.flags,
            registers: self.state.registers,
        }
    }
}

// Re-exported so callers that hand-encode bytes (see the integration
// test below, and `jxcl-golden`'s vector fixtures) can name faults
// without duplicating this crate's opcode table.
pub use state::ExecutionFault as Fault;

#[cfg(test)]
mod tests {
    use super::*;

    fn enc_movi(rd: u8, imm: u64) -> Vec<u8> {
        let mut b = vec![0x02, rd];
        b.extend_from_slice(&imm.to_le_bytes());
        b
    }
    fn enc_rr(opcode: u8, rd: u8, rs: u8) -> Vec<u8> {
        vec![opcode, rd, rs]
    }
    fn enc_r(opcode: u8, rd: u8) -> Vec<u8> {
        vec![opcode, rd]
    }
    fn enc_branch(opcode: u8, disp: i32) -> Vec<u8> {
        let mut b = vec![opcode];
        b.extend_from_slice(&disp.to_le_bytes());
        b
    }
    fn enc_none(opcode: u8) -> Vec<u8> {
        vec![opcode]
    }

    #[test]
    fn add_two_constants_and_halt() {
        let mut program = Vec::new();
        program.extend(enc_movi(1, 10));
        program.extend(enc_movi(2, 20));
        program.extend(enc_rr(0x10, 1, 2)); // ADD R1, R2
        program.extend(enc_none(0x60)); // HALT

        let mut sim = Simulator::new(program);
        let result = sim.run();
        assert!(result.halted);
        assert!(result.fault.is_none());
        assert_eq!(result.register(1), 30);
        assert_eq!(result.register(2), 20);
    }

    #[test]
    fn divide_by_zero_faults_and_preserves_operand() {
        let mut program = Vec::new();
        program.extend(enc_movi(1, 5));
        program.extend(enc_movi(2, 0));
        program.extend(enc_rr(0x16, 1, 2)); // DIV R1, R2
        program.extend(enc_none(0x60));

        let mut sim = Simulator::new(program);
        let result = sim.run();
        assert!(result.halted);
        assert_eq!(result.fault.as_deref(), Some("DivideByZero"));
        assert_eq!(result.register(1), 5);
    }

    /// Real integration test: a small but non-trivial program (a
    /// countdown loop summing 1..=5 into R1, mirroring
    /// `crates/jxcl/tests/golden_vectors.rs`'s `sum_one_to_five_via_loop`
    /// vector) hand-encoded directly to bytes and run to completion,
    /// exercising MOVI, ADD, DEC, CMP, the signed-conditional branch
    /// JG, and HALT end-to-end through `Simulator::new`/`run`.
    ///
    /// `jxcl-assembler` is still a placeholder as of this batch (see
    /// this crate's module doc comment), so there is no assembler to
    /// produce this program from source text; the bytes are hand-encoded
    /// against this crate's own real opcode table instead, exactly as
    /// this batch's instructions anticipate.
    #[test]
    fn loop_sums_one_to_five_end_to_end() {
        // MOVI R1, 0
        // MOVI R2, 5
        // loop:
        //   ADD R1, R2
        //   DEC R2
        //   CMP R2, R0
        //   JG loop
        // HALT
        let mut program = Vec::new();
        program.extend(enc_movi(1, 0)); // offset 0..10
        program.extend(enc_movi(2, 5)); // offset 10..20
        let loop_start = program.len() as i64; // 20
        program.extend(enc_rr(0x10, 1, 2)); // ADD R1, R2   offset 20..23
        program.extend(enc_r(0x1A, 2)); // DEC R2           offset 23..25
        program.extend(enc_rr(0x40, 2, 0)); // CMP R2, R0   offset 25..28
        let jg_offset = program.len() as i64; // 28
        let jg_len = 5i64;
        let pc_after_jg = jg_offset + jg_len; // 33
        let disp = (loop_start - pc_after_jg) as i32; // -13
        program.extend(enc_branch(0x59, disp)); // JG loop  offset 28..33
        program.extend(enc_none(0x60)); // HALT             offset 33..34

        let mut sim = Simulator::new(program);
        let result = sim.run();

        assert!(result.halted, "program did not halt: {result:?}");
        assert!(result.fault.is_none(), "unexpected fault: {result:?}");
        assert_eq!(result.register(1), 15); // 5+4+3+2+1
        assert_eq!(result.register(2), 0);
    }

    #[test]
    fn step_single_instruction_at_a_time() {
        let mut program = Vec::new();
        program.extend(enc_movi(1, 7));
        program.extend(enc_none(0x60));
        let mut sim = Simulator::new(program);
        assert!(!sim.is_halted());
        let r1 = sim.step();
        assert_eq!(r1.register(1), 7);
        assert!(!r1.halted);
        let r2 = sim.step();
        assert!(r2.halted);
    }

    #[test]
    fn with_memory_size_rejects_undersized_memory() {
        let program = enc_none(0x60);
        let err = Simulator::with_memory_size(program, 0).unwrap_err();
        assert!(err.contains("smaller than"));
    }

    #[test]
    fn unknown_opcode_faults_cleanly() {
        let program = vec![0xEE];
        let mut sim = Simulator::new(program);
        let result = sim.run();
        assert_eq!(result.fault.as_deref(), Some("InvalidOpcode"));
    }
}
