# jxcl-opcodes

The single authoritative opcode registry: ids, mnemonics, operand-shape metadata.

## Architecture

**Owns:** The opcode table -- no other crate may define or duplicate opcode identifiers.

**Category:** isa · **Source:** extraction:jxcl/src/isa/opcodes.rs

## Public API

`Opcode`, `OPCODE_TABLE`, `opcode_by_mnemonic`

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-constants`

External crates:

*(none)*

## Testing

Planned test kinds: unit, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
