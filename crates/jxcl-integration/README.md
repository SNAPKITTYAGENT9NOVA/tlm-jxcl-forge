# jxcl-integration

An integration-test-only crate exercising the full stack end-to-end: assemble, run in jxcl-simulator, seal/store in pq-cache, attest with pq-error-proof.

## Architecture

**Owns:** The canonical whole-workspace integration tests.

**Category:** security · **Source:** new

## Public API

`(test-only crate)`

## Dependencies

Workspace crates:

- `jxcl-assembler`
- `jxcl-simulator`
- `pq-cache`
- `pq-error-proof`

External crates:

*(none)*

## Testing

Planned test kinds: integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
