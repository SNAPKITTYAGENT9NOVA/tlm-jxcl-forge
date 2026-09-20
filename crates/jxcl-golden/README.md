# jxcl-golden

The golden-vector format, loader, and canonical vector files used as the ISA conformance baseline.

## Architecture

**Owns:** GoldenVector and tests/vectors/.

**Category:** debug · **Source:** extraction:jxcl/tests/golden_vectors.rs

## Public API

`GoldenVector`, `load_vectors`, `run_vector`

## Dependencies

Workspace crates:

- `jxcl-simulator`
- `jxcl-assembler`

External crates:

- `serde`

## Testing

Planned test kinds: golden, conformance.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
