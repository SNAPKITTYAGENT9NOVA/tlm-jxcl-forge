# jxcl-alu

Arithmetic/logic unit semantics: add/sub/mul/div/shift/bitwise, with flag updates.

## Architecture

**Owns:** All arithmetic semantics -- no other crate performs ALU-equivalent computation independently.

**Category:** execution · **Source:** extraction:jxcl/src/alu.rs

## Public API

`Alu`, `Alu::execute`

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-flags`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, property, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
