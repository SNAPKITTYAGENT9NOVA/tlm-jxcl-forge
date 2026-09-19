# jxcl-page-table

A single-level virtual-to-physical page table with map/unmap/translate and page-fault on unmapped access.

## Architecture

**Owns:** PageTable.

**Category:** memory · **Source:** new

## Public API

`PageTable`

## Dependencies

Workspace crates:

- `jxcl-address-space`
- `jxcl-exceptions`

External crates:

*(none)*

## Testing

Planned test kinds: unit, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
