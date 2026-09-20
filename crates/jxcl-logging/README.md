# jxcl-logging

Shared tracing/logging initialization used by the CLI and the network services.

## Architecture

**Owns:** init_logging() and the RUST_LOG-driven subscriber setup convention.

**Category:** foundation · **Source:** extraction:jxcl/src/main.rs,photo-cache-service/src/lib.rs

## Public API

`init_logging`

## Dependencies

Workspace crates:

*(none)*

External crates:

- `tracing`
- `tracing-subscriber`

## Testing

Planned test kinds: unit.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
