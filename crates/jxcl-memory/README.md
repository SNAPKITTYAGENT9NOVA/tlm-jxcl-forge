# jxcl-memory

Raw byte-addressable memory storage and bounds-checked byte-level access.

## Architecture

**Owns:** The Memory struct -- the only place raw bytes are stored.

**Category:** memory · **Source:** extraction:jxcl/src/memory.rs

## Public API

`Memory`

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-errors`
- `jxcl-endian`

External crates:

*(none)*

## Testing

Planned test kinds: unit, property, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
