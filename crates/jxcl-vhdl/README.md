# jxcl-vhdl

Renders the same jxcl-hdl AST to VHDL text, proving the AST is backend-agnostic.

## Architecture

**Owns:** The VHDL text backend.

**Category:** hardware · **Source:** new

## Public API

`render_vhdl`

## Dependencies

Workspace crates:

- `jxcl-hdl`

External crates:

*(none)*

## Testing

Planned test kinds: unit, golden.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
