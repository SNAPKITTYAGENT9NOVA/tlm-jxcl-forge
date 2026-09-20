# jxcl-pipeline

A staged fetch/decode/execute/writeback pipeline-stage model used to report realistic per-instruction cycle costs, distinct from the (non-pipelined) reference execution semantics.

## Purpose

This crate provides a classical 4-stage in-order scalar pipeline implementation for cycle-cost estimation. Rather than naively assuming one cycle per instruction, the pipeline models the effect of instruction-level parallelism and pipeline fill/drain behavior on total execution cycles. Each instruction progresses through four stages (Fetch → Decode → Execute → Writeback) over four consecutive cycles, allowing multiple instructions to be in flight simultaneously and producing more realistic per-instruction cycle-cost estimates than simpler models.

## Architecture

**Owns:** `PipelineStage` (enum for the four stages) and `Pipeline` (the state machine struct).

**Category:** execution · **Source:** new

## Public API

- `PipelineStage`: An enum identifying each of the four pipeline stages (Fetch, Decode, Execute, Writeback).
- `Pipeline`: The main struct managing in-flight instructions across stages, advancing them each cycle, and reporting cycle counts.

### Key Methods

- `Pipeline::new()`: Create an empty pipeline.
- `insert_instruction(instr)`: Insert an instruction at the Fetch stage; returns `false` if Fetch is occupied.
- `advance_cycle() -> u64`: Advance all instructions one stage and return the total cycle count.
- `instruction_at_stage(stage) -> Option<Instruction>`: Query which instruction (if any) is in a given stage.
- `stop_accepting_instructions()`: Signal end-of-program; pipeline continues draining until empty.
- `is_drained() -> bool`: Return `true` if the pipeline is empty and no longer accepting instructions.
- `instruction_cycle_cost(instr) -> u64`: Static method returning the per-opcode cycle cost from `jxcl-cycle-model`.

## Implementation Notes

### Simplifying Assumptions

This is a **cost-estimation model**, not a cycle-accurate simulator. The following simplifications are intentional:

1. **No hazards or stalls**: Data hazards (RAW, WAW, WAR), control hazards (branch misprediction), and structural hazards (resource contention) are not modeled. Instructions advance freely through each stage every cycle.

2. **One instruction per stage per cycle**: The pipeline holds at most one instruction per stage. This models a scalar (non-superscalar) in-order pipeline.

3. **Deterministic per-opcode costs**: Cycle costs come directly from `jxcl-cycle-model::cycle_cost()`, independent of instruction type or position in the pipeline. The pipeline does not simulate multi-cycle operations or memory latency.

4. **No branch prediction or speculative execution**: Branches are treated as normal instructions; no branch predictor is modeled.

### Fill and Drain Behavior

- **Pipeline fill**: When instructions are inserted at the Fetch stage, they advance one stage per cycle. A single instruction takes four cycles to progress from Fetch to completion (one cycle per stage).
- **Pipeline drain**: After `stop_accepting_instructions()` is called, the pipeline continues advancing remaining instructions until all stages are empty.
- **Cycle counting**: Cycles are counted during both fill and drain phases, and only stop being counted once the pipeline is completely drained and accepting is stopped.

## Dependencies

Workspace crates:

- `jxcl-instructions`: For the `Instruction` type and related definitions.
- `jxcl-cycle-model`: For the `cycle_cost()` function and per-opcode cost tables.

External crates:

*(none)*

## Testing

15 unit tests covering:

- Pipeline initialization and emptiness.
- Instruction insertion and fetch-stage occupation.
- Single instruction progression through all four stages.
- Multiple instruction fill behavior (up to full 4-instruction pipeline).
- Pipeline drain behavior after `stop_accepting_instructions()`.
- In-order drain of multiple instructions.
- Cycle count accumulation during fill, steady-state, and drain phases.
- Instruction cycle-cost lookup via the cycle-cost model.
- Handling of many instructions with repeated insert/advance cycles.
- Default and Display trait implementations.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
