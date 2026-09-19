# jxcl-config

Environment-variable configuration parsing helpers (typed env_or, hex-seed decoding) shared by the CLI and services.

## Architecture

**Owns:** env_or/env_or_parse/decode_hex_seed and the 'log the resolved value only if not secret' convention.

**Category:** foundation · **Source:** extraction:photo-cache-service/src/lib.rs,pq-crypto/src/lib.rs

## Public API

`env_or`, `env_or_parse`, `decode_hex_seed`

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
