# jxcl

Backward-compatible facade preserving the original crate's public API and `jxcl` binary name over the newly split crates.

## Architecture

**Owns:** Nothing new -- re-exports the ISA/execution/toolchain crates under their original module paths (isa::, encoding::, alu::, memory::, machine::, execution::, control::, binary::, validator::, assembler::, disassembler::, debugger::).

**Category:** isa · **Source:** facade

## Public API

`(re-exports of the pre-expansion public API, unchanged)`

## Dependencies

Workspace crates:

- `jxcl-opcodes`
- `jxcl-registers`
- `jxcl-flags`
- `jxcl-operands`
- `jxcl-instructions`
- `jxcl-encoding`
- `jxcl-decoding`
- `jxcl-alu`
- `jxcl-memory`
- `jxcl-machine`
- `jxcl-execution`
- `jxcl-control`
- `jxcl-binary`
- `jxcl-instruction-validation`
- `jxcl-assembler`
- `jxcl-disassembler`
- `jxcl-debugger`
- `jxcl-cli`

External crates:

*(none)*

## Testing

Planned test kinds: integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
