# pq-crypto

Backward-compatible facade preserving the original crate's public API over the newly split kem/kdf/aead/envelope/keyring/rotation crates.

## Architecture

**Owns:** Nothing new -- re-exports seal/open/KeyPair/Envelope/KeyRing/KeyStatus/Error under their original paths.

**Category:** crypto · **Source:** facade

## Public API

`(re-exports of the pre-expansion public API, unchanged)`

## Dependencies

Workspace crates:

- `pq-kem`
- `pq-kdf`
- `pq-aead`
- `pq-envelope`
- `pq-keyring`
- `pq-rotation`

External crates:

*(none)*

## Testing

Planned test kinds: integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
