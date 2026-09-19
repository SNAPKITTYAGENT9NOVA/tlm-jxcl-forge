# pq-attestation

Combines sealing a value (pq-envelope) with a verifiable attestation (pq-proof-types) that it was sealed under a specific, named key version, in one call.

## Architecture

**Owns:** seal_with_attestation / open_with_attestation.

**Category:** crypto · **Source:** new

## Public API

`seal_with_attestation`, `open_with_attestation`

## Dependencies

Workspace crates:

- `pq-envelope`
- `pq-proof-types`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
