# jxcl-instruction-validation

Per-instruction operand-legality checks for the JXCL ISA.

## Purpose

Validates instruction operands according to ISA rules:
- Register operands are in valid range (0..REGISTER_COUNT)
- Immediates are within appropriate ranges
- Memory addressing modes use valid base registers

This is the single source of truth for instruction legality, used by both the decoder and static binary validator to ensure consistent validation rules.

## Public API

- `validate_instruction(&Instruction) -> Result<(), Error>` - Validate an instruction's operands

## Implementation Notes

- All register operands must be in range `0..32` (the configured REGISTER_COUNT)
- Immediate values use the full `u64` range for 64-bit immediates
- Memory displacement fields (`i32`) are always valid (no range constraints)
- Validation is the single source of truth shared between the decoder and validator

## Testing

Planned test kinds: unit, boundary, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
