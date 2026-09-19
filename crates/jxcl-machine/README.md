# jxcl-machine

Architectural state: the register file, flags, and program counter as one cohesive Machine struct.

## Architecture

**Owns:** The Machine struct -- the single authoritative representation of architectural state.

**Category:** execution · **Source:** extraction:jxcl/src/machine.rs

## Public API

`Machine`

## Dependencies

Workspace crates:

- `jxcl-registers`
- `jxcl-flags`
- `jxcl-types`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
