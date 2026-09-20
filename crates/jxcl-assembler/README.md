# jxcl-assembler

The two-pass assembler: source text to an object file (plus a convenience path assembling and linking a single file straight to a binary, preserving the original single-step CLI workflow).

## Architecture

**Owns:** assemble(source) -> ObjectFile.

**Category:** toolchain · **Source:** extraction:jxcl/src/assembler/mod.rs

## Public API

`assemble`, `assemble_to_binary`

## Dependencies

Workspace crates:

- `jxcl-lexer`
- `jxcl-parser`
- `jxcl-object`
- `jxcl-symbols`
- `jxcl-relocations`
- `jxcl-encoding`
- `jxcl-instructions`
- `jxcl-linker`
- `jxcl-debug-info`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
