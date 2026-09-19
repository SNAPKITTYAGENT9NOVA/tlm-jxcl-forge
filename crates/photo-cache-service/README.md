# photo-cache-service

The two axum binaries (server, server-cached) mirroring the original Express demo, rebuilt on jxcl-service/jxcl-http/jxcl-network/pq-storage/pq-policy (existing public behavior preserved).

## Architecture

**Owns:** The GET /photos and GET /healthz handlers and the two binaries.

**Category:** network · **Source:** extraction:photo-cache-service/src/*

## Public API

`(binaries: server, server-cached)`

## Dependencies

Workspace crates:

- `jxcl-service`
- `jxcl-http`
- `jxcl-network`
- `pq-cache`
- `pq-policy`
- `pq-crypto`

External crates:

- `axum`
- `reqwest`
- `tokio`

## Testing

Planned test kinds: unit, integration (live Redis).

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
