# jxcl-profiler

Aggregates a trace, the cycle model, and the cache model into run statistics (instruction count, cycles, cache hit rate).

## Architecture

**Owns:** ProfileReport.

**Category:** debug · **Source:** new

## Public API

`ProfileReport`, `profile_run`

## Dependencies

Workspace crates:

- `jxcl-trace`
- `jxcl-cycle-model`
- `jxcl-cache-model`

External crates:

*(none)*

## Testing

Planned test kinds: unit, determinism.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
