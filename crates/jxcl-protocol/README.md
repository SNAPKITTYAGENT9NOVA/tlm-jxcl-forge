# jxcl-protocol

A serde-serializable request/response protocol for remote jxcl-machine control: assemble, run, return trace/final state.

## Architecture

**Owns:** the Request/Response wire types.

**Category:** network · **Source:** new

## Public API

`Request`, `Response`

## Dependencies

Workspace crates:

- `jxcl-simulator`
- `jxcl-trace`

External crates:

- `serde`

## Testing

Planned test kinds: unit, serialization.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
