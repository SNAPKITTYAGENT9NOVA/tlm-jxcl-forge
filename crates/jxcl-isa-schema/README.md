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

The crate includes 15 unit tests covering:
- Schema generation from live ISA
- Spec conformance checking
- Detection of architectural parameter mismatches
- JSON serialization and deserialization
- Schema round-trip preservation
- Opcode space calculations and sanity checks

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
