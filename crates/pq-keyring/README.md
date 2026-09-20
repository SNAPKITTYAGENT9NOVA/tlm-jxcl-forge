# pq-keyring

The KeyRing data structure: an indexed set of key-pair entries.

## Architecture

**Owns:** KeyRing's storage and lookup (insert/get/iterate) -- not rotation policy, see pq-rotation.

**Category:** crypto · **Source:** extraction:pq-crypto/src/lib.rs

## Public API

`KeyRing`

## Dependencies

Workspace crates:

- `pq-envelope`

External crates:

*(none)*

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
