# jxcl-cycle-model

A per-opcode cycle-cost table and accumulator giving a deterministic total-cycle count for a run.

## Architecture

**Owns:** CYCLE_COST_TABLE and CycleCounter.

**Category:** execution · **Source:** new

## Public API

`CycleCounter`, `cycle_cost`

## Dependencies

Workspace crates:

- `jxcl-opcodes`

External crates:

*(none)*

## Testing

Planned test kinds: unit, determinism.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
