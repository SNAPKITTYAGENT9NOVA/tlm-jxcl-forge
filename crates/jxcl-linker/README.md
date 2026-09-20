# jxcl-linker

Resolves symbols and applies relocations across one or more object files into a single linked binary container.

## Architecture

**Owns:** link(&[ObjectFile]) -> BinaryContainer.

**Category:** toolchain · **Source:** new

## Public API

`link`

## Dependencies

Workspace crates:

- `jxcl-object`
- `jxcl-symbols`
- `jxcl-relocations`
- `jxcl-binary`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
