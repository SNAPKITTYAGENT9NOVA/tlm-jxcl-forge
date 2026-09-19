# jxcl-hardware

Top-level facade integrating the HDL AST, both text backends, and the toy synthesis pass behind one emit API.

## Architecture

**Owns:** emit_verilog/emit_vhdl/emit_netlist entry points for the jxcl core datapath.

**Category:** hardware · **Source:** new

## Public API

`emit_verilog`, `emit_vhdl`, `emit_netlist`

## Dependencies

Workspace crates:

- `jxcl-rtl`
- `jxcl-verilog`
- `jxcl-vhdl`
- `jxcl-netlist`
- `jxcl-synthesis`

External crates:

*(none)*

## Testing

Planned test kinds: integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
