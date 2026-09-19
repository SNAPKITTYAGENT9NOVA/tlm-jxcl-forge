# jxcl-stack

Typed stack-pointer push/pop with overflow/underflow checking for call/return semantics.

## Architecture

**Owns:** Stack::push/pop and its bounds checks.

**Category:** memory · **Source:** new

## Public API

`Stack`

## Dependencies

Workspace crates:

- `jxcl-load-store`
- `jxcl-exceptions`
- `jxcl-registers`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
