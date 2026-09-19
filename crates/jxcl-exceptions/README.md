# jxcl-exceptions

The machine's exception model: illegal opcode, misaligned access, division by zero, and the trap-handling hook.

## Architecture

**Owns:** The Exception enum and the trap dispatch contract.

**Category:** execution · **Source:** new

## Public API

`Exception`, `TrapHandler`

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
