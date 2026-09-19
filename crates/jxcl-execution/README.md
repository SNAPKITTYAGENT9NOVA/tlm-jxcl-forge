# jxcl-execution

The fetch/decode/execute loop: owns instruction execution, wiring together decoding, dispatch, the ALU, memory, and exceptions.

## Architecture

**Owns:** step(&mut Machine) -- the only place an instruction is actually executed.

**Category:** execution · **Source:** extraction:jxcl/src/execution.rs

## Public API

`step`, `run`

## Dependencies

Workspace crates:

- `jxcl-machine`
- `jxcl-alu`
- `jxcl-control`
- `jxcl-decoding`
- `jxcl-memory`
- `jxcl-dispatch`
- `jxcl-exceptions`
- `jxcl-load-store`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, property, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
