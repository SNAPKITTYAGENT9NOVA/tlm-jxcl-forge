# jxcl-security

Cross-cutting secret-redaction utilities, generalizing the two independent redaction implementations found in photo-cache-service (Redis URL) and pq-sql-vault (ADO connection string) into one shared, tested implementation.

## Architecture

**Owns:** redact_secret_bearing_string and the shared redaction convention.

**Category:** security · **Source:** new

## Public API

`redact_secret_bearing_string`

## Dependencies

Workspace crates:

- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
