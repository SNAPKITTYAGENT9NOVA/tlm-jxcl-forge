# pq-proof-verifier

A verifier facade that looks up the right scheme via pq-proof-registry and verifies, so callers never touch arkworks types directly.

## Architecture

**Owns:** verify_attestation(scheme_id, ...).

**Category:** proof · **Source:** new

## Public API

`verify_attestation`

## Dependencies

Workspace crates:

- `pq-proof-types`
- `pq-proof-registry`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
