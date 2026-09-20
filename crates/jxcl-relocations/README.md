# jxcl-relocations

Relocation entry types and the patch-in-place logic that fixes up an encoded instruction once a symbol's final address is known.

## Architecture

**Owns:** Relocation and apply_relocation.

**Category:** toolchain · **Source:** new

## Public API

`Relocation`, `apply_relocation`

## Dependencies

Workspace crates:

- `jxcl-symbols`
- `jxcl-encoding`
- `jxcl-bitops`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
