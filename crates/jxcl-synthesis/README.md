# jxcl-synthesis

A toy (explicitly documented, non-production) structural synthesis pass lowering a subset of the HDL AST into primitive-gate netlist form.

## Architecture

**Owns:** synthesize(Module) -> Netlist.

**Category:** hardware · **Source:** new

## Public API

`synthesize`

## Dependencies

Workspace crates:

- `jxcl-hdl`
- `jxcl-netlist`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
