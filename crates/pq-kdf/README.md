# pq-kdf

HKDF-SHA256 expansion of a KEM shared secret into an AES-256 key, with fixed domain separation.

## Architecture

**Owns:** derive_aes_key.

**Category:** crypto · **Source:** extraction:pq-crypto/src/lib.rs

## Public API

`derive_aes_key`

## Dependencies

Workspace crates:

*(none)*

External crates:

- `hkdf`
- `sha2`

## Testing

Planned test kinds: unit, known-answer.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
