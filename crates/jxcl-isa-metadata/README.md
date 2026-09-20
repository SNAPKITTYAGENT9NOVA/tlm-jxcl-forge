# jxcl-isa-metadata

Queryable descriptive metadata about the running ISA build (opcode count, register count, build flags) used by `jxcl inspect` and the debugger.

## Architecture

**Owns:** IsaMetadata::describe().

**Category:** isa · **Source:** new

## Public API

`IsaMetadata`, `describe`

## Dependencies

Workspace crates:

- `jxcl-constants`
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
