# pq-object-store

Chunked/streamed large-blob storage built on top of any SealedStore, splitting values above a size threshold into sealed chunks with a manifest.

## Architecture

**Owns:** put_object/get_object and the chunk-manifest format.

**Category:** storage · **Source:** new

## Public API

`put_object`, `get_object`

## Dependencies

Workspace crates:

- `pq-storage`
- `pq-envelope`

External crates:

*(none)*

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
