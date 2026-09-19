# jxcl-pipeline

A staged fetch/decode/execute/writeback pipeline-stage model used to report realistic per-instruction cycle costs, distinct from the (non-pipelined) reference execution semantics.

## Architecture

**Owns:** PipelineStage and the stage-advance state machine.

**Category:** execution · **Source:** new

## Public API

`PipelineStage`, `Pipeline`

## Dependencies

Workspace crates:

- `jxcl-instructions`
- `jxcl-cycle-model`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
