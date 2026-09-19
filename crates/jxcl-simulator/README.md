# jxcl-simulator

An embeddable top-level simulation API (load + run in one call) for using jxcl as a library from other Rust programs/services.

## Architecture

**Owns:** Simulator::new/run.

**Category:** debug · **Source:** new

## Public API

`Simulator`, `RunResult`

## Dependencies

Workspace crates:

- `jxcl-loader`
- `jxcl-machine`
- `jxcl-execution`
- `jxcl-memory-map`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
