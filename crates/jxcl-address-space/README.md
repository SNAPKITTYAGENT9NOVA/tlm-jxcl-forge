# jxcl-address-space

Named memory regions (code/data/stack) with permission flags layered over raw storage.

## Architecture

**Owns:** AddressSpace and Region (base, len, permissions).

**Category:** memory · **Source:** new

## Public API

`AddressSpace`, `Region`, `Permission`

## Dependencies

Workspace crates:

- `jxcl-memory`
- `jxcl-types`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
