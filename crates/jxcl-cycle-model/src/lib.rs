// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A per-opcode cycle-cost table and accumulator giving a deterministic total-cycle count for a run.
//!
//! Owns: CYCLE_COST_TABLE and CycleCounter. Provides a deterministic cycle accounting
//! model independent of the (non-pipelined) reference execution semantics.
//!
//! The cycle cost for each instruction is fixed and deterministic. This model
//! enables realistic per-instruction cycle costs to be reported while the
//! actual execution remains non-pipelined for simplicity (spec §9, §12).
#![forbid(unsafe_code)]

use jxcl_opcodes::Mnemonic;

/// The cycle cost (in machine cycles) for each instruction type.
/// All costs are single-cycle for simplicity; memory operations would
/// be higher in a real implementation.
pub const CYCLE_COST_TABLE: &[(Mnemonic, u64)] = &[
    // Data movement (1 cycle)
    (Mnemonic::Mov, 1),
    (Mnemonic::Movi, 1),
    (Mnemonic::Load, 1),
    (Mnemonic::Store, 1),
    (Mnemonic::Push, 1),
    (Mnemonic::Pop, 1),
    (Mnemonic::Lea, 1),
    // Integer arithmetic (1 cycle)
    (Mnemonic::Add, 1),
    (Mnemonic::Sub, 1),
    (Mnemonic::Adc, 1),
    (Mnemonic::Sbc, 1),
    (Mnemonic::Mul, 1),
    (Mnemonic::Mulh, 1),
    (Mnemonic::Div, 1),
    (Mnemonic::Rem, 1),
    (Mnemonic::Neg, 1),
    (Mnemonic::Inc, 1),
    (Mnemonic::Dec, 1),
    // Logical (1 cycle)
    (Mnemonic::And, 1),
    (Mnemonic::Or, 1),
    (Mnemonic::Xor, 1),
    (Mnemonic::Not, 1),
    (Mnemonic::Nand, 1),
    (Mnemonic::Nor, 1),
    (Mnemonic::Xor3, 1),
    // Shift / rotate (1 cycle)
    (Mnemonic::Shl, 1),
    (Mnemonic::Shr, 1),
    (Mnemonic::Sar, 1),
    (Mnemonic::Rol, 1),
    (Mnemonic::Ror, 1),
    // Comparison (1 cycle)
    (Mnemonic::Cmp, 1),
    (Mnemonic::Test, 1),
    // Control flow (1 cycle)
    (Mnemonic::Jmp, 1),
    (Mnemonic::Call, 1),
    (Mnemonic::Ret, 1),
    (Mnemonic::Jz, 1),
    (Mnemonic::Jnz, 1),
    (Mnemonic::Jc, 1),
    (Mnemonic::Jnc, 1),
    (Mnemonic::Jl, 1),
    (Mnemonic::Jle, 1),
    (Mnemonic::Jg, 1),
    (Mnemonic::Jge, 1),
    // System (1 cycle)
    (Mnemonic::Nop, 1),
    (Mnemonic::Halt, 1),
    (Mnemonic::Trap, 1),
    (Mnemonic::Sys, 1),
    // Memory / atomic (1 cycle)
    (Mnemonic::Cas, 1),
    (Mnemonic::Xchg, 1),
    (Mnemonic::Fence, 1),
];

/// Return the cycle cost for a given mnemonic.
/// Defaults to 1 cycle if the mnemonic is not found in the table.
pub const fn cycle_cost(mnemonic: Mnemonic) -> u64 {
    // Linear search through the cost table
    let mut i = 0;
    while i < CYCLE_COST_TABLE.len() {
        if CYCLE_COST_TABLE[i].0 as u8 == mnemonic as u8 {
            return CYCLE_COST_TABLE[i].1;
        }
        i += 1;
    }
    // Default: 1 cycle
    1
}

/// A cycle counter for tracking total machine cycles during execution.
/// Deterministic and independent of the actual execution timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CycleCounter {
    total_cycles: u64,
}

impl CycleCounter {
    /// Create a new cycle counter starting at zero.
    pub const fn new() -> Self {
        CycleCounter { total_cycles: 0 }
    }

    /// Add cycles to the counter.
    pub fn add_cycles(&mut self, cycles: u64) {
        self.total_cycles = self.total_cycles.wrapping_add(cycles);
    }

    /// Record an instruction execution with its deterministic cycle cost.
    pub fn execute_instruction(&mut self, mnemonic: Mnemonic) {
        let cost = cycle_cost(mnemonic);
        self.add_cycles(cost);
    }

    /// Get the total number of cycles executed so far.
    pub const fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    /// Reset the cycle counter to zero.
    pub fn reset(&mut self) {
        self.total_cycles = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_cost_for_mnemonics() {
        assert_eq!(cycle_cost(Mnemonic::Add), 1);
        assert_eq!(cycle_cost(Mnemonic::Mov), 1);
        assert_eq!(cycle_cost(Mnemonic::Jmp), 1);
        assert_eq!(cycle_cost(Mnemonic::Nop), 1);
    }

    #[test]
    fn cycle_counter_accumulates() {
        let mut counter = CycleCounter::new();
        assert_eq!(counter.total_cycles(), 0);

        counter.add_cycles(1);
        assert_eq!(counter.total_cycles(), 1);

        counter.add_cycles(2);
        assert_eq!(counter.total_cycles(), 3);
    }

    #[test]
    fn cycle_counter_wraps_on_overflow() {
        let mut counter = CycleCounter::new();
        counter.total_cycles = u64::MAX;
        counter.add_cycles(1);
        assert_eq!(counter.total_cycles(), 0);
    }

    #[test]
    fn execute_instruction_adds_cost() {
        let mut counter = CycleCounter::new();
        counter.execute_instruction(Mnemonic::Add);
        assert_eq!(counter.total_cycles(), 1);

        counter.execute_instruction(Mnemonic::Mov);
        assert_eq!(counter.total_cycles(), 2);

        counter.execute_instruction(Mnemonic::Jmp);
        assert_eq!(counter.total_cycles(), 3);
    }

    #[test]
    fn cycle_counter_reset() {
        let mut counter = CycleCounter::new();
        counter.add_cycles(42);
        assert_eq!(counter.total_cycles(), 42);

        counter.reset();
        assert_eq!(counter.total_cycles(), 0);
    }

    #[test]
    fn cycle_counter_determinism() {
        let mut counter1 = CycleCounter::new();
        let mut counter2 = CycleCounter::new();

        for _ in 0..100 {
            counter1.execute_instruction(Mnemonic::Add);
            counter2.execute_instruction(Mnemonic::Add);
        }

        assert_eq!(counter1.total_cycles(), counter2.total_cycles());
        assert_eq!(counter1.total_cycles(), 100);
    }
}
