# jxcl-synthesis

A toy (explicitly documented, non-production) structural synthesis pass lowering a subset of the HDL AST into primitive-gate netlist form.

## Honesty boundary

This is **not** a production synthesis tool. There is no `iverilog`,
`verilator`, or `yosys` in this environment (see
`docs/HARDWARE_LIMITATIONS.md` at the workspace root), so `synthesize`'s
output is never checked by feeding it to a real logic synthesizer --
only by this crate's own unit tests and `jxcl-netlist::Netlist::validate`'s
structural check.

`synthesize` recognizes **exactly one shape**: a module whose one
top-level `case` statement selects on a single 1-bit identifier, has
exactly two arms (for selector values `0` and `1`, no `default`), and
whose arm bodies are `target = identifier;` assignments to the same set
of targets in both arms -- i.e. a bank of independent 2:1 multiplexers.
`jxcl-hdl`'s own canonical `mux2` example is the smallest instance of
this shape. Anything outside it (a wider selector, a `default` arm, a
literal or bit-slice right-hand side, mismatched targets between arms)
is rejected with a descriptive `SynthesisError` rather than silently
producing something that looks plausible but isn't equivalent to the
source module.

## Architecture

**Owns:** `synthesize(&Module) -> Result<Netlist, SynthesisError>`.

**Category:** hardware · **Source:** new

## Public API

`synthesize`, `SynthesisError`

## Dependencies

Workspace crates:

- `jxcl-hdl`
- `jxcl-netlist`

External crates:

*(none)*

## Testing

9 unit tests, including the required "2-input mux case statement ->
expected gate structure" case (`mux2_synthesizes_to_a_single_mux_gate`)
plus explicit rejection tests for every shape outside this pass's
documented scope. Run with `cargo test -p jxcl-synthesis`.

## Status

`implemented`.
