# jxcl-disassembler

Disassembles a binary/object file back to readable assembly text, symbolizing addresses when debug info is present.

## Architecture

**Owns:** disassemble(bytes) -> String.

**Category:** toolchain · **Source:** extraction:jxcl/src/disassembler.rs

## Public API

`disassemble`

## Dependencies

Workspace crates:

- `jxcl-decoding`
- `jxcl-instructions`
- `jxcl-symbols`
- `jxcl-debug-info`

External crates:

*(none)*

## Testing

Planned test kinds: unit, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
