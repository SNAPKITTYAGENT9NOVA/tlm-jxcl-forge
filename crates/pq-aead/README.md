# pq-aead

AES-256-GCM authenticated encryption/decryption of the plaintext under the derived key.

## Architecture

**Owns:** aead_encrypt/aead_decrypt.

**Category:** crypto · **Source:** extraction:pq-crypto/src/lib.rs

## Public API

`aead_encrypt`, `aead_decrypt`

## Dependencies

Workspace crates:

*(none)*

External crates:

- `aes-gcm`

## Testing

Planned test kinds: unit, tamper, wrong-key.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
