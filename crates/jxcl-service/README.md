# jxcl-service

Generic service scaffolding (graceful shutdown, health-check endpoint pattern, startup logging) extracted from photo-cache-service's two binaries' shared boilerplate.

## Architecture

**Owns:** serve_with_graceful_shutdown and the healthz handler pattern.

**Category:** network · **Source:** extraction:photo-cache-service/src/bin/*.rs

## Public API

`serve_with_graceful_shutdown`, `healthz_handler`

## Dependencies

Workspace crates:

- `jxcl-http`
- `jxcl-logging`
- `jxcl-config`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
