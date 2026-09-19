# jxcl-load-store

Typed, sign-extension-aware load/store helpers (u8/u16/u32/u64 and signed variants) between raw memory and the execution engine.

## Architecture

**Owns:** load_u8/16/32/64, store_u8/16/32/64 and their signed counterparts.

**Category:** memory · **Source:** new

## Public API

`load_u8`, `load_u16`, `load_u32`, `load_u64`, `store_u8`, `store_u16`, `store_u32`, `store_u64`

## Dependencies

Workspace crates:

- `jxcl-memory`
- `jxcl-endian`
- `jxcl-exceptions`

External crates:

*(none)*

## Testing

Planned test kinds: unit, property.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
