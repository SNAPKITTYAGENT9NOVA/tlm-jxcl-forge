# jxcl-cli

The `jxcl` command-line interface: asm/disasm/run/inspect/validate subcommands.

## Architecture

**Owns:** The CLI argument parsing and subcommand dispatch (main.rs's logic).

**Category:** toolchain · **Source:** extraction:jxcl/src/main.rs

## Public API

`main`

## Dependencies

Workspace crates:

- `jxcl-assembler`
- `jxcl-disassembler`
- `jxcl-loader`
- `jxcl-simulator`
- `jxcl-debugger`
- `jxcl-isa-metadata`
- `jxcl-instruction-validation`
- `jxcl-config`
- `jxcl-logging`

External crates:

*(none)*

## Testing

Planned test kinds: integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
