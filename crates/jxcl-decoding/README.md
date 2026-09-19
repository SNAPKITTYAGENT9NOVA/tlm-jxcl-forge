# jxcl-decoding

Decodes an Instruction from its binary wire format -- the exact inverse of jxcl-encoding.

## Architecture

**Owns:** decode(bytes) -> Instruction; the only place instruction bit layout is read.

**Category:** isa · **Source:** extraction:jxcl/src/encoding/decoder.rs

## Public API

`decode`

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

Planned test kinds: unit, property, golden, fuzz.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
