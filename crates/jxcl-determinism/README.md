# jxcl-determinism

Machine state snapshot/restore and the determinism property-test harness.

## Architecture

**Owns:** Snapshot and the Snapshot/Restore trait.

**Category:** debug · **Source:** extraction:jxcl/tests/property_tests.rs

## Public API

`Snapshot`, `SnapshotRestore`

## Dependencies

Workspace crates:

- `jxcl-machine`
- `jxcl-memory`

External crates:

*(none)*

## Testing

Planned test kinds: unit, property, determinism.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
