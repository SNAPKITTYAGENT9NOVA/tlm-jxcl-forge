# jxcl-instruction-validation

Per-instruction operand-legality checks (register range, immediate range, addressing-mode compatibility with the opcode) shared by the decoder and the static binary validator.

## Architecture

**Owns:** validate_instruction(&Instruction) -> Result<(), Error> -- the single source of truth for 'is this a legal instruction', so the decoder and jxcl-validator never diverge.

**Category:** isa · **Source:** new

## Public API

`validate_instruction`

## Dependencies

Workspace crates:

- `jxcl-instructions`
- `jxcl-operands`
- `jxcl-registers`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
