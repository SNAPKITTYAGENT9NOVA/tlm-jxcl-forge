# jxcl-heap

A simple bump/free-list allocator operating within an address space, for programs needing dynamic memory.

## Architecture

**Owns:** Heap::alloc/free.

**Category:** memory · **Source:** new

## Public API

`Heap`

## Dependencies

Workspace crates:

- `jxcl-address-space`
- `jxcl-load-store`
- `jxcl-exceptions`

External crates:

*(none)*

## Testing

Planned test kinds: unit, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
