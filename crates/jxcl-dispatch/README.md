# jxcl-dispatch

Table-driven opcode-to-handler dispatch, extracted from the fetch/decode/execute loop's match statement so dispatch can be tested and extended independently of execution semantics.

## Architecture

**Owns:** The dispatch table (Opcode -> handler fn pointer).

**Category:** execution · **Source:** new

## Public API

`DispatchTable`, `DispatchTable::lookup`

## Dependencies

Workspace crates:

- `jxcl-instructions`
- `jxcl-opcodes`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
