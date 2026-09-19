# jxcl-memory-map

The concrete default memory layout (where code/stack/heap/MMIO live) consumed by the loader and machine setup.

## Architecture

**Owns:** MemoryMap, the default layout constant.

**Category:** memory · **Source:** new

## Public API

`MemoryMap`, `DEFAULT_MEMORY_MAP`

## Dependencies

Workspace crates:

- `jxcl-address-space`
- `jxcl-constants`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
