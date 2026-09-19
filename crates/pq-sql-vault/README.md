# pq-sql-vault

A SQL Server-backed store for sealed values (existing public API preserved; now implements pq-storage::SealedStore and applies its schema via pq-migration).

## Architecture

**Owns:** The SQL Server-specific connection/get/set_with_ttl implementation and stored-procedure calls.

**Category:** storage · **Source:** extraction:pq-sql-vault/src/lib.rs

## Public API

`SqlVault`, `SealedStore for SqlVault`

## Dependencies

Workspace crates:

- `pq-storage`
- `pq-envelope`
- `pq-keyring`
- `pq-migration`

External crates:

- `tiberius`

## Testing

Planned test kinds: unit, integration (ignored, no live SQL Server).

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
