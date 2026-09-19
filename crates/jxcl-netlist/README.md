# jxcl-netlist

A structural gate-level netlist intermediate representation (primitive gates + wires).

## Honesty boundary

This crate is a plain Rust data structure for a netlist made of four
primitive gate kinds (`And`, `Or`, `Not`, `Mux`) connected by named
wires. It does **not** simulate gate behavior, drive timing, or talk to
a logic synthesizer. There is no `iverilog`, `verilator`, or `yosys` in
this environment (see `docs/HARDWARE_LIMITATIONS.md` at the workspace
root). `Netlist::validate` checks only *structural* well-formedness
(gate arity matches its kind; no net driven by two gates) -- never
electrical or functional correctness.

## Architecture

**Owns:** The `Netlist`/`Gate`/`Wire` types, and the primitive
`GateKind` enum (`And`/`Or`/`Not`/`Mux`).

**Category:** hardware · **Source:** new

## Public API

`Netlist`, `Gate`, `Wire`, `GateKind`, `NetlistError`,
`Netlist::validate`

## Dependencies

Workspace crates:

- `jxcl-hdl`

External crates:

*(none)*

## Testing

7 unit tests: gate-arity constants, constructor well-formedness,
malformed-gate rejection, multiply-driven-net rejection, and incremental
wire/gate construction. Run with `cargo test -p jxcl-netlist`.

## Status

`implemented`.
