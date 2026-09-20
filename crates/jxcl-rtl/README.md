# jxcl-rtl

The RTL description of jxcl's core datapath (opcode decoder, ALU, register file) generated as a jxcl-hdl AST directly from jxcl-opcodes/jxcl-constants/jxcl-alu -- the mechanically-verifiable software/hardware bridge required by docs/RTL_CONTRACT.md.

## Honesty boundary

This crate builds a hardware-description **AST**. It does not simulate
anything, and nothing here is checked against a real Verilog/VHDL
toolchain -- there is no `iverilog`, `verilator`, or `yosys` in this
environment (see `docs/HARDWARE_LIMITATIONS.md` at the workspace root).
What *is* checked, mechanically, is that `build_decoder_module`'s
generated `case` statement cannot silently drift from
`jxcl-opcodes::all_defs()`'s real opcode table -- see
`decoder_case_arms_match_opcode_table_exactly` below, the mechanical
proof `docs/RTL_CONTRACT.md` requires.

**`jxcl-alu` status at the time this crate was written:** `jxcl-alu` is
still a placeholder crate (no `Alu` type, no operation table to
iterate) -- it is being built concurrently by a sibling batch.
`build_alu_module` is therefore a documented **minimal placeholder**
module: it captures only the width contract (two `WORD_BITS` operands
in, one `WORD_BITS` result out, an operation-selector input), not a
real per-operation case statement. Once `jxcl-alu` lands with a real
operation enum/table, `build_alu_module` should be rewritten to
generate one case arm per real ALU operation, exactly mirroring how
`build_decoder_module` is generated from `jxcl-opcodes` today.

`build_register_file_module` likewise has no internal behavioral
statements: `jxcl-hdl`'s AST has no indexed-array/memory-cell
construct, so only the interface width contract (address ports sized
from `jxcl_constants::REGISTER_COUNT`, data ports sized from
`jxcl_constants::WORD_BITS`) is generated and checked.

## Architecture

**Owns:** `build_decoder_module`/`build_alu_module`/`build_register_file_module`.

**Category:** hardware · **Source:** new

## Public API

`build_decoder_module`, `build_alu_module`, `build_register_file_module`

## Dependencies

Workspace crates:

- `jxcl-hdl`
- `jxcl-opcodes`
- `jxcl-constants`
- `jxcl-alu` (placeholder as of this writing -- see above)

External crates:

*(none)*

## Testing

7 unit tests, including the critical mechanical cross-check
`decoder_case_arms_match_opcode_table_exactly` (the generated decoder's
non-default case arms match `jxcl-opcodes::all_defs()` exactly, by
count and by opcode id) plus width-contract checks for the register
file and ALU placeholder ports. Run with `cargo test -p jxcl-rtl`.

## Status

`implemented` (decoder, register-file interface); ALU module is an
explicitly documented placeholder pending real `jxcl-alu`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
