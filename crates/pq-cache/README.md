# pq-cache

A Redis-backed cache whose entries are sealed with pq-crypto before they reach Redis (existing public API preserved; now also implements pq-storage::SealedStore).

## Architecture

**Owns:** The Redis-specific connection/get/set_with_ttl implementation.

**Category:** storage · **Source:** extraction:pq-cache/src/lib.rs

## Public API

`EncryptedCache`, `SealedStore for EncryptedCache`

## Dependencies

Workspace crates:

- `pq-storage`
- `pq-envelope`
- `pq-keyring`

External crates:

- `redis`

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
