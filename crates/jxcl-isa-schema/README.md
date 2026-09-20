# jxcl-isa-schema

A serde-serializable schema of the JXCL ISA with conformance checking against documentation.

## Purpose

Provides a machine-readable, serde-serializable schema of the JXCL ISA:
- Architectural parameters (word width, address width, opcode width, binary version)
- Opcode table metadata (addressable space, assigned opcodes)
- Register file description (GP register count, special registers)
- Flags register description (flag names)

Can be checked against documentation to ensure spec/code consistency.

## Public API

- `IsaSchema::generate() -> IsaSchema` - Generate schema from live ISA registries
- `IsaSchema::check_against_spec() -> Result<(), String>` - Verify against documented ISA parameters
- `IsaSchema::to_json(&self) -> Result<String>` - Serialize to JSON
- `IsaSchema::from_json(json: &str) -> Result<IsaSchema>` - Deserialize from JSON

## Implementation Notes

- The schema is mechanical - generated entirely from live authoritative sources
- Documented values (64-bit words, 32 registers, 256 opcode space, etc.) are embedded as invariants
- `check_against_spec()` verifies the documented ISA parameters match the code
- Full JSON round-trip compatibility via serde

## Testing

Planned test kinds: unit, conformance.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
