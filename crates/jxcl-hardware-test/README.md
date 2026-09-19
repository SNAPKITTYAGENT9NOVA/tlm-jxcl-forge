# jxcl-hardware-test

Golden-file regression tests for generated Verilog/VHDL/netlist, plus the mechanical width/opcode-count cross-check against the ISA schema.

## Architecture

**Owns:** The hardware golden files and the ISA-vs-RTL conformance check.

**Category:** hardware · **Source:** new

## Public API

`(test-only crate)`

## Dependencies

Workspace crates:

- `jxcl-hardware`
- `jxcl-isa-schema`

External crates:

*(none)*

## Testing

Planned test kinds: golden, conformance.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
