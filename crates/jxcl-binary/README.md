# jxcl-binary

The final linked-executable container format: header, version, code/data sections.

## Architecture

**Owns:** The binary container's byte layout.

**Category:** toolchain · **Source:** extraction:jxcl/src/binary.rs

## Public API

`BinaryContainer`

## Dependencies

Workspace crates:

- `jxcl-bytes`
- `jxcl-endian`
- `jxcl-errors`
- `jxcl-isa-versioning`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
