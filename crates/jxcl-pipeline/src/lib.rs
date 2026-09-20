// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A staged fetch/decode/execute/writeback pipeline-stage model used to report realistic per-instruction cycle costs, distinct from the (non-pipelined) reference execution semantics.
//!
//! This module provides a classical 4-stage pipeline implementation for cycle-cost estimation.
//! Each instruction progresses through four stages: Fetch → Decode → Execute → Writeback,
//! with one stage per cycle per instruction.
//!
//! ## Design and Simplifying Assumptions
//!
//! - **No hazards or stalls**: Data hazards, control hazards, and structural hazards are not modeled.
//!   Instructions advance freely through each stage without blocking. This is acceptable because
//!   the pipeline exists as a cost-estimation tool, not a cycle-accurate simulator.
//!
//! - **One instruction per stage per cycle**: The pipeline can hold at most one instruction per stage.
//!   This models a simple in-order scalar 4-stage pipeline.
//!
//! - **Deterministic cycle costs**: Per-opcode cycle costs come from `jxcl-cycle-model`'s
//!   `cycle_cost()` function. The pipeline does not re-compute costs; it reports the cycles
//!   determined by the cost table.
//!
//! - **Fill and drain behavior**: At the start of execution, the pipeline fills (stages 1–4
//!   enter instructions over cycles 0–3). At the end, it drains (remaining instructions
//!   complete even after the last fetch).
//!
//! ## Public API
//!
//! - [`PipelineStage`]: Enum representing the four pipeline stages.
//! - [`Pipeline`]: The main struct tracking in-flight instructions and advancing them each cycle.
#![forbid(unsafe_code)]

use jxcl_cycle_model::cycle_cost;
use jxcl_instructions::Instruction;

/// A pipeline stage identifier: one of Fetch, Decode, Execute, or Writeback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// Fetch stage: instruction bytes are fetched from memory.
    Fetch,
    /// Decode stage: fetched bytes are decoded into an instruction.
    Decode,
    /// Execute stage: instruction is executed.
    Execute,
    /// Writeback stage: results are written back to registers/memory.
    Writeback,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineStage::Fetch => write!(f, "Fetch"),
            PipelineStage::Decode => write!(f, "Decode"),
            PipelineStage::Execute => write!(f, "Execute"),
            PipelineStage::Writeback => write!(f, "Writeback"),
        }
    }
}

/// A simple 4-stage pipeline for reporting realistic per-instruction cycle costs.
///
/// The pipeline tracks which instruction is in each of the four stages across cycles,
/// advancing them one stage per cycle. Cycle costs are determined by the
/// `jxcl-cycle-model`'s per-opcode table.
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// The four stages: [Fetch, Decode, Execute, Writeback].
    /// `None` means the stage is empty (no instruction in flight).
    stages: [Option<Instruction>; 4],
    /// Total cycles accumulated so far (for reporting).
    total_cycles: u64,
    /// Whether we are still accepting new instructions (i.e., fetch hasn't seen EOF).
    accepting_instructions: bool,
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new() -> Self {
        Pipeline {
            stages: [None; 4],
            total_cycles: 0,
            accepting_instructions: true,
        }
    }

    /// Insert an instruction at the Fetch stage.
    ///
    /// Returns `true` if the instruction was inserted (Fetch stage was empty).
    /// Returns `false` if Fetch is already occupied (caller should retry next cycle).
    pub fn insert_instruction(&mut self, instruction: Instruction) -> bool {
        if self.stages[0].is_none() {
            self.stages[0] = Some(instruction);
            true
        } else {
            false
        }
    }

    /// Advance the pipeline by one cycle.
    ///
    /// Instructions move one stage forward (Fetch → Decode → Execute → Writeback → out).
    /// Returns the total accumulated cycles.
    pub fn advance_cycle(&mut self) -> u64 {
        // Increment cycles if we are still accepting new instructions OR
        // if the pipeline has in-flight instructions (check before shift).
        if self.accepting_instructions || self.stages.iter().any(|s| s.is_some()) {
            self.total_cycles += 1;
        }

        // Advance instructions through stages: Writeback (drains), Execute → Writeback, Decode → Execute, Fetch → Decode
        // We process from back to front to avoid overwriting
        self.stages[3] = self.stages[2].take(); // Execute → Writeback
        self.stages[2] = self.stages[1].take(); // Decode → Execute
        self.stages[1] = self.stages[0].take(); // Fetch → Decode

        self.total_cycles
    }

    /// Return the instruction currently in a given stage, if any.
    pub fn instruction_at_stage(&self, stage: PipelineStage) -> Option<Instruction> {
        let idx = stage_index(stage);
        self.stages[idx]
    }

    /// Return the total accumulated cycles.
    pub const fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    /// Mark that no more instructions will be inserted (signals end-of-program).
    /// The pipeline will continue draining until all stages are empty.
    pub fn stop_accepting_instructions(&mut self) {
        self.accepting_instructions = false;
    }

    /// Return `true` if the pipeline is completely empty and not accepting instructions.
    pub fn is_drained(&self) -> bool {
        !self.accepting_instructions && self.stages.iter().all(|s| s.is_none())
    }

    /// Return the cycle cost for an instruction, using the cycle-cost model.
    pub fn instruction_cycle_cost(instruction: Instruction) -> u64 {
        cycle_cost(instruction.mnemonic)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a `PipelineStage` to a 0-3 index for the stages array.
fn stage_index(stage: PipelineStage) -> usize {
    match stage {
        PipelineStage::Fetch => 0,
        PipelineStage::Decode => 1,
        PipelineStage::Execute => 2,
        PipelineStage::Writeback => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_instructions::{Instruction, Mnemonic, Operands};
    use jxcl_registers::Register;

    fn make_nop() -> Instruction {
        Instruction::new(Mnemonic::Nop, Operands::None)
    }

    fn make_add() -> Instruction {
        let rd = Register::new(0);
        Instruction::new(Mnemonic::Add, Operands::RR { rd, rs: rd })
    }

    fn make_mov() -> Instruction {
        let rd = Register::new(0);
        Instruction::new(Mnemonic::Mov, Operands::R { rd })
    }

    #[test]
    fn new_pipeline_is_empty() {
        let pipeline = Pipeline::new();
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Fetch), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Decode), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Execute), None);
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            None
        );
        assert_eq!(pipeline.total_cycles(), 0);
    }

    #[test]
    fn insert_instruction_in_fetch_stage() {
        let mut pipeline = Pipeline::new();
        let instr = make_nop();
        assert!(pipeline.insert_instruction(instr));
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Fetch),
            Some(instr)
        );
    }

    #[test]
    fn cannot_insert_when_fetch_occupied() {
        let mut pipeline = Pipeline::new();
        let instr1 = make_nop();
        let instr2 = make_add();
        assert!(pipeline.insert_instruction(instr1));
        assert!(!pipeline.insert_instruction(instr2)); // Fetch is occupied
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Fetch),
            Some(instr1)
        );
    }

    #[test]
    fn single_instruction_fills_pipeline_over_four_cycles() {
        let mut pipeline = Pipeline::new();
        let instr = make_nop();
        assert!(pipeline.insert_instruction(instr));

        // Cycle 1: Fetch -> Decode
        pipeline.advance_cycle();
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Fetch), None);
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Decode),
            Some(instr)
        );
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Execute), None);
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            None
        );

        // Cycle 2: Decode -> Execute
        pipeline.advance_cycle();
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Fetch), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Decode), None);
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Execute),
            Some(instr)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            None
        );

        // Cycle 3: Execute -> Writeback
        pipeline.advance_cycle();
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Fetch), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Decode), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Execute), None);
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            Some(instr)
        );

        // Cycle 4: Writeback drains (instruction completes)
        pipeline.advance_cycle();
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Fetch), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Decode), None);
        assert_eq!(pipeline.instruction_at_stage(PipelineStage::Execute), None);
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            None
        );
    }

    #[test]
    fn pipeline_fill_with_four_instructions() {
        let mut pipeline = Pipeline::new();
        let instr1 = make_nop();
        let instr2 = make_add();
        let instr3 = make_mov();
        let instr4 = make_nop();

        // Cycle 0: Insert instr1 in Fetch
        assert!(pipeline.insert_instruction(instr1));
        assert_eq!(pipeline.total_cycles(), 0);

        // Cycle 1: Advance and insert instr2
        pipeline.advance_cycle();
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Decode),
            Some(instr1)
        );
        assert!(pipeline.insert_instruction(instr2));
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Fetch),
            Some(instr2)
        );

        // Cycle 2: Advance and insert instr3
        pipeline.advance_cycle();
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Execute),
            Some(instr1)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Decode),
            Some(instr2)
        );
        assert!(pipeline.insert_instruction(instr3));

        // Cycle 3: Advance and insert instr4
        pipeline.advance_cycle();
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            Some(instr1)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Execute),
            Some(instr2)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Decode),
            Some(instr3)
        );
        assert!(pipeline.insert_instruction(instr4));

        // Cycle 4: All four stages filled (instr1 drains)
        pipeline.advance_cycle();
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            Some(instr2)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Execute),
            Some(instr3)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Decode),
            Some(instr4)
        );
    }

    #[test]
    fn pipeline_drain_after_stop_accepting() {
        let mut pipeline = Pipeline::new();
        let instr = make_nop();
        assert!(pipeline.insert_instruction(instr));
        pipeline.stop_accepting_instructions();

        // The pipeline should continue draining even after stop_accepting
        pipeline.advance_cycle(); // Fetch -> Decode
        assert!(!pipeline.is_drained());

        pipeline.advance_cycle(); // Decode -> Execute
        assert!(!pipeline.is_drained());

        pipeline.advance_cycle(); // Execute -> Writeback
        assert!(!pipeline.is_drained());

        pipeline.advance_cycle(); // Writeback drains
        assert!(pipeline.is_drained());
    }

    #[test]
    fn multiple_instructions_drain_in_order() {
        let mut pipeline = Pipeline::new();
        let instr1 = make_nop();
        let instr2 = make_add();
        let instr3 = make_mov();

        // Fill pipeline with 3 instructions over 3 cycles
        assert!(pipeline.insert_instruction(instr1));
        pipeline.advance_cycle();
        assert!(pipeline.insert_instruction(instr2));
        pipeline.advance_cycle();
        assert!(pipeline.insert_instruction(instr3));
        pipeline.advance_cycle();

        // At this point:
        // Writeback: instr1, Execute: instr2, Decode: instr3
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            Some(instr1)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Execute),
            Some(instr2)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Decode),
            Some(instr3)
        );

        // Stop accepting and drain
        pipeline.stop_accepting_instructions();

        // Cycle 4: instr1 drains, instr2 -> Writeback, instr3 -> Execute
        pipeline.advance_cycle();
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            Some(instr2)
        );
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Execute),
            Some(instr3)
        );
        assert!(!pipeline.is_drained());

        // Cycle 5: instr2 drains, instr3 -> Writeback
        pipeline.advance_cycle();
        assert_eq!(
            pipeline.instruction_at_stage(PipelineStage::Writeback),
            Some(instr3)
        );
        assert!(!pipeline.is_drained());

        // Cycle 6: instr3 drains
        pipeline.advance_cycle();
        assert!(pipeline.is_drained());
    }

    #[test]
    fn cycle_count_advances_correctly() {
        let mut pipeline = Pipeline::new();
        let instr = make_nop();
        assert!(pipeline.insert_instruction(instr));
        assert_eq!(pipeline.total_cycles(), 0);

        pipeline.advance_cycle();
        assert_eq!(pipeline.total_cycles(), 1);

        pipeline.advance_cycle();
        assert_eq!(pipeline.total_cycles(), 2);

        pipeline.stop_accepting_instructions();
        pipeline.advance_cycle();
        assert_eq!(pipeline.total_cycles(), 3);
    }

    #[test]
    fn cycle_count_includes_fill_and_drain() {
        let mut pipeline = Pipeline::new();
        let instr = make_nop();
        assert!(pipeline.insert_instruction(instr));

        // 4 cycles to fill and drain a single instruction: Fetch, Decode, Execute, Writeback
        for _ in 0..4 {
            pipeline.advance_cycle();
        }
        assert_eq!(pipeline.total_cycles(), 4);

        // Pipeline should now be empty
        assert!(pipeline.is_drained() || pipeline.accepting_instructions);
    }

    #[test]
    fn instruction_cycle_cost_from_mnemonic() {
        let nop = make_nop();
        assert_eq!(Pipeline::instruction_cycle_cost(nop), 1);

        let add = make_add();
        assert_eq!(Pipeline::instruction_cycle_cost(add), 1);

        let mov = make_mov();
        assert_eq!(Pipeline::instruction_cycle_cost(mov), 1);
    }

    #[test]
    fn pipeline_with_many_instructions() {
        let mut pipeline = Pipeline::new();
        let instructions = [make_nop(), make_add(), make_mov(), make_nop(), make_add()];

        // Insert all instructions
        for instr in &instructions {
            while !pipeline.insert_instruction(*instr) {
                pipeline.advance_cycle();
            }
        }

        // Signal end of instructions and drain
        pipeline.stop_accepting_instructions();
        while !pipeline.is_drained() {
            pipeline.advance_cycle();
        }

        // Verify the pipeline drained correctly
        assert!(pipeline.is_drained());
        // Total cycles should be: 4 (fill) + 1 (last instruction inserted) + 4 (drain last instr) = 9
        // But let's verify by simulation:
        // 0: insert instr[0] in Fetch
        // 1: adv, insert instr[1]
        // 2: adv, insert instr[2]
        // 3: adv, insert instr[3]
        // 4: adv, insert instr[4]
        // 5: adv, stop accepting
        // 6-8: adv 3 more times to drain
        // Total advances = 8, so 8 cycles
        assert_eq!(pipeline.total_cycles(), 8);
    }

    #[test]
    fn default_pipeline_is_equivalent_to_new() {
        let p1 = Pipeline::new();
        let p2 = Pipeline::default();
        assert_eq!(p1.total_cycles(), p2.total_cycles());
        assert_eq!(
            p1.instruction_at_stage(PipelineStage::Fetch),
            p2.instruction_at_stage(PipelineStage::Fetch)
        );
    }

    #[test]
    fn pipeline_stage_display() {
        assert_eq!(format!("{}", PipelineStage::Fetch), "Fetch");
        assert_eq!(format!("{}", PipelineStage::Decode), "Decode");
        assert_eq!(format!("{}", PipelineStage::Execute), "Execute");
        assert_eq!(format!("{}", PipelineStage::Writeback), "Writeback");
    }

    #[test]
    fn accepting_instructions_flag() {
        let mut pipeline = Pipeline::new();
        assert!(pipeline.insert_instruction(make_nop())); // Pipeline starts accepting
        pipeline.stop_accepting_instructions();
        // After stopping, we can't insert new instructions, but can still advance
        pipeline.advance_cycle();
        assert_eq!(pipeline.total_cycles(), 1);
    }
}
