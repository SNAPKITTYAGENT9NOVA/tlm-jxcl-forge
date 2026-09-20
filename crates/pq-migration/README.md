# pq-migration

Applies pq-sql-vault's sql/*.sql migration files in order against a live connection and tracks which have been applied.

## Architecture

**Owns:** run_migrations.

**Category:** storage · **Source:** new

## Public API

`run_migrations`, `Migration`

## Dependencies

Workspace crates:

*(none)*

External crates:

- `tiberius`

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
