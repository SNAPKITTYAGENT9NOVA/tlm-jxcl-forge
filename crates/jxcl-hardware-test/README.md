# jxcl-hardware-test

Golden-file regression tests for generated Verilog/VHDL/netlist, plus the mechanical width/opcode-count cross-check against the ISA schema.

## Honesty boundary

This is a test-only crate. Nothing in it simulates the generated RTL or
feeds it to a real Verilog/VHDL/synthesis toolchain -- there is no
`iverilog`, `verilator`, or `yosys` in this environment (see
`docs/HARDWARE_LIMITATIONS.md` at the workspace root). "Passes" here
means: matches a checked-in golden file under `tests/golden/`, or
matches a structural fact mechanically computed from
`jxcl-opcodes`/`jxcl-constants`'s real tables/constants -- not "was
hardware-verified."

**`jxcl-isa-schema` status at the time this crate was written:**
`jxcl-isa-schema` (this crate's other declared dependency) is still a
placeholder -- no `IsaSchema` type or `generate`/`check_against_spec`
methods -- as of this writing. `tests/isa_conformance.rs` therefore
cross-checks the generated decoder directly against
`jxcl-opcodes::all_defs()` and `jxcl-constants`'s real constants,
instead of going through `jxcl-isa-schema`. Once `jxcl-isa-schema`
lands, that test should be switched to compare against
`IsaSchema::generate()` instead, without changing what it actually
asserts (a real `IsaSchema` would itself be generated from the same two
crates).

## Architecture

**Owns:** The hardware golden files (`tests/golden/`) and the
ISA-vs-RTL conformance check.

**Category:** hardware · **Source:** new

## Public API

Test-only crate; `render_netlist_text` is exposed only so a `Netlist`
can be golden-file compared the same way rendered Verilog/VHDL text is.

## Dependencies

Workspace crates:

- `jxcl-hardware`
- `jxcl-isa-schema` (placeholder as of this writing -- see above)
- `jxcl-hdl`
- `jxcl-netlist`
- `jxcl-opcodes`
- `jxcl-constants`
- `jxcl-rtl`

External crates:

*(none)*

## Testing

10 tests total: 2 unit tests for `render_netlist_text`, 4 golden-file
regression tests (`tests/golden_regression.rs`, comparing
`jxcl-hardware`'s emitted Verilog/VHDL and a synthesized netlist against
checked-in files under `tests/golden/`), and 4 ISA conformance tests
(`tests/isa_conformance.rs`). Run with `cargo test -p
jxcl-hardware-test`.

## Status

`implemented`, with the `jxcl-isa-schema` cross-check as a documented
direct-table fallback pending that crate's real implementation.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
