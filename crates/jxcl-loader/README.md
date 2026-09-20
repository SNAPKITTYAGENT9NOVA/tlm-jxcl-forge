# jxcl-loader

Loads a linked binary container into a memory image laid out per jxcl-memory-map, ready to run.

## Architecture

**Owns:** load(BinaryContainer) -> (Memory, entry_point).

**Category:** toolchain · **Source:** new

## Public API

`load`

## Dependencies

Workspace crates:

- `jxcl-binary`
- `jxcl-memory-map`
- `jxcl-memory`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
