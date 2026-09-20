# jxcl-trace

The per-step execution trace record format and its writer/reader, reused by the profiler and replay.

## Architecture

**Owns:** TraceStep and TraceLog.

**Category:** debug · **Source:** new

## Public API

`TraceStep`, `TraceLog`

## Dependencies

Workspace crates:

- `jxcl-machine`
- `jxcl-instructions`

External crates:

*(none)*

## Testing

Planned test kinds: unit, serialization.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
