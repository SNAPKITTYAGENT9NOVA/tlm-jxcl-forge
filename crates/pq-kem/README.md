# pq-kem

ML-KEM-768 (NIST FIPS 203) key generation and encapsulation/decapsulation.

## Architecture

**Owns:** KeyPair and the raw KEM step.

**Category:** crypto · **Source:** extraction:pq-crypto/src/lib.rs

## Public API

`KeyPair`, `EncapsulationKey`, `DecapsulationKey`

## Dependencies

Workspace crates:

*(none)*

External crates:

- `ml-kem`

## Testing

Planned test kinds: unit, known-answer.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
