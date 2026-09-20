# pq-envelope

The sealed-value wire format: key version, KEM ciphertext, nonce, AEAD ciphertext.

## Architecture

**Owns:** Envelope and its to_bytes/from_bytes wire format -- the only place the envelope byte layout is defined.

**Category:** crypto · **Source:** extraction:pq-crypto/src/lib.rs

## Public API

`Envelope`, `seal`, `open`

## Dependencies

Workspace crates:

- `pq-kem`
- `pq-kdf`
- `pq-aead`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary, known-answer.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
