# jxcl-http

Shared HTTP client/server helpers (timeout wrapper, error-to-status mapping) extracted from photo-cache-service's duplicated per-binary logic.

## Architecture

**Owns:** with_timeout and AppError-to-http-status mapping.

**Category:** network · **Source:** extraction:photo-cache-service/src/lib.rs

## Public API

`with_timeout`, `error_to_status`

## Dependencies

Workspace crates:

- `jxcl-errors`

External crates:

- `axum`
- `reqwest`

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
