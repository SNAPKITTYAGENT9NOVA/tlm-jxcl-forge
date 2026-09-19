# jxcl-rtl

The RTL description of jxcl's core datapath (opcode decoder, ALU, register file) generated as a jxcl-hdl AST directly from jxcl-opcodes/jxcl-constants/jxcl-alu -- the mechanically-verifiable software/hardware bridge required by docs/RTL_CONTRACT.md.

## Architecture

**Owns:** build_decoder_module/build_alu_module/build_register_file_module.

**Category:** hardware · **Source:** new

## Public API

`build_decoder_module`, `build_alu_module`, `build_register_file_module`

## Dependencies

Workspace crates:

- `jxcl-hdl`
- `jxcl-opcodes`
- `jxcl-constants`
- `jxcl-alu`

External crates:

*(none)*

## Testing

Planned test kinds: unit, conformance.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
