# jxcl-isa-versioning

ISA/binary-format version negotiation so old binaries fail closed against an incompatible newer decoder rather than silently misdecoding.

## Architecture

**Owns:** IsaVersion, its embedding in the binary container header, and the compatibility check.

**Category:** isa · **Source:** new

## Public API

`IsaVersion`, `IsaVersion::is_compatible_with`

## Dependencies

Workspace crates:

- `jxcl-constants`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
