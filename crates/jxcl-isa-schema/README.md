# jxcl-isa-schema

A serde-serializable schema of the ISA generated from jxcl-opcodes/jxcl-constants/jxcl-registers/jxcl-flags, plus a mechanical cross-check against the numbers documented in docs/ISA_SPEC.md.

## Architecture

**Owns:** IsaSchema and the spec-vs-code conformance check that ISA_SPEC.md's stated widths/opcode-count match the generated schema.

**Category:** isa · **Source:** new

## Public API

`IsaSchema`, `IsaSchema::generate`, `IsaSchema::check_against_spec`

## Dependencies

Workspace crates:

- `jxcl-opcodes`
- `jxcl-constants`
- `jxcl-registers`
- `jxcl-flags`

External crates:

- `serde`

## Testing

Planned test kinds: unit, conformance.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
