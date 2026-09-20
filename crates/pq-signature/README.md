# pq-signature

ML-DSA (NIST FIPS 204 / Dilithium) signing and verification -- a second, complementary post-quantum primitive (authenticity, alongside pq-kem's confidentiality).

## Architecture

**Owns:** SigningKey/VerifyingKey and sign/verify.

**Category:** crypto · **Source:** new

## Public API

`SigningKey`, `VerifyingKey`, `sign`, `verify`

## Dependencies

Workspace crates:

*(none)*

External crates:

- `ml-dsa`

## Testing

Planned test kinds: unit, known-answer, tamper, wrong-key.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
