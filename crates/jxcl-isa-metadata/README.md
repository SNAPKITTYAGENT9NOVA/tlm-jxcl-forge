# jxcl-isa-metadata

Queryable descriptive metadata about the JXCL ISA build.

## Purpose

Provides comprehensive, queryable metadata about the running ISA:
- Architectural parameters (word width, address width, opcode width)
- Register count and special register names
- Opcode statistics (assigned opcodes vs. total space)
- Architecture identifier and binary format version

Used by inspection tools (e.g., `jxcl inspect`) and debuggers to report ISA capabilities.

## Public API

- `IsaMetadata::describe() -> IsaMetadata` - Generate metadata from live ISA registries and constants
- `IsaMetadata::validate() -> Result<(), String>` - Verify internal consistency
- `IsaMetadata::summary() -> String` - Human-readable summary
- `describe()` - Convenience function equivalent to `IsaMetadata::describe()`

## Implementation Notes

- Metadata is generated entirely from the authoritative `jxcl-constants` and `jxcl-opcodes` registries
- The `validate()` method checks consistency (e.g., word width == address width == 64 bits)
- Opcode space is derived from opcode width via `2^opcode_width`
- Assigned opcodes count is a live query of the registry

## Testing

The crate includes 9 unit tests covering:
- Metadata describes the live ISA correctly
- Self-consistency validation passes
- Opcode space calculations are correct
- Assigned opcodes match the registry
- Validation catches inconsistencies
- Display formatting works correctly
