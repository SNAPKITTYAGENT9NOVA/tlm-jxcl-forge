# jxcl-cache-model

A direct-mapped/set-associative cache simulation layered over memory, for profiling hit/miss behavior.

## Architecture

**Owns:** CacheModel and its hit/miss accounting.

**Category:** memory · **Source:** new

## Public API

`CacheModel`

## Dependencies

Workspace crates:

- `jxcl-memory`
- `jxcl-address-space`

External crates:

*(none)*

## Testing

Planned test kinds: unit, determinism.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
