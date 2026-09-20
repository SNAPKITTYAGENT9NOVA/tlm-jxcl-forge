# pq-rotation

Key-rotation policy: the Active/DecryptOnly/Retired lifecycle and the set_active/retire transition rules, kept separate from the KeyRing data structure itself.

## Architecture

**Owns:** KeyStatus and the rotation state-transition rules.

**Category:** crypto · **Source:** extraction:pq-crypto/src/lib.rs

## Public API

`KeyStatus`, `set_active`, `retire`

## Dependencies

Workspace crates:

- `pq-keyring`

External crates:

*(none)*

## Testing

Planned test kinds: unit, rotation.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
