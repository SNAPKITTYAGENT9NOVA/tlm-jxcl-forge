# jxcl-replay

Deterministically replays a recorded trace to reconstruct machine state at any step, without re-running the original inputs.

## Architecture

**Owns:** Replay::seek_to_step.

**Category:** debug · **Source:** new

## Public API

`Replay`

## Dependencies

Workspace crates:

- `jxcl-trace`
- `jxcl-machine`
- `jxcl-determinism`

External crates:

*(none)*

## Testing

Planned test kinds: unit, determinism.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
