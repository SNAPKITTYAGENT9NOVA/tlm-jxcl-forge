# jxcl-encoding

Encodes an Instruction to its binary wire format.

## Architecture

**Owns:** encode(Instruction) -> bytes; the only place instruction bit layout is written.

**Category:** isa · **Source:** extraction:jxcl/src/encoding/encoder.rs

## Public API

`encode`

## Dependencies

Workspace crates:

- `jxcl-instructions`
- `jxcl-bitops`
- `jxcl-endian`
- `jxcl-bytes`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, property, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
