# jxcl-object

A relocatable object-file format (sections + symbol table + relocation table) distinct from the final linked binary container.

## Architecture

**Owns:** ObjectFile serialization/deserialization.

**Category:** toolchain · **Source:** new

## Public API

`ObjectFile`

## Dependencies

Workspace crates:

- `jxcl-bytes`
- `jxcl-symbols`
- `jxcl-relocations`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
