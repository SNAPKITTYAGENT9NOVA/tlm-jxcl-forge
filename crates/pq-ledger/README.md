# pq-ledger

A tamper-evident, hash-chained audit ledger recording proof attestations in order, built on pq-journal.

## Architecture

**Owns:** Ledger::record/verify_chain.

**Category:** storage · **Source:** new

## Public API

`Ledger`

## Dependencies

Workspace crates:

- `pq-journal`
- `pq-proof-types`

External crates:

*(none)*

## Testing

Planned test kinds: unit, tamper.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
