//! Queryable descriptive metadata about the running ISA build (opcode
//! count, register count, build flags) used by `jxcl inspect` and the
//! debugger.
//!
//! Owns: `IsaMetadata` struct and the `describe()` function that
//! generates it from the authoritative ISA registries.
#![forbid(unsafe_code)]

use jxcl_constants::{
    ADDRESS_WIDTH_BITS, ARCHITECTURE_ID, BINARY_VERSION, OPCODE_BITS, OPCODE_SPACE,
    REGISTER_COUNT, REGISTER_WIDTH_BITS, WORD_BITS,
};
use jxcl_opcodes::assigned_opcode_count;

#[cfg(test)]
use jxcl_opcodes::all_defs;

/// Comprehensive queryable metadata about the running ISA build, used
/// by inspection tools, debuggers, and documentation generation.
///
/// This aggregates architectural constants and registry statistics into
/// one authoritative description of the ISA's shape and capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsaMetadata {
    /// Width of an architectural word, in bits.
    pub word_width_bits: u32,
    /// Width of an address, in bits.
    pub address_width_bits: u32,
    /// Width of the opcode field, in bits.
    pub opcode_width_bits: u32,
    /// Total number of addressable opcode values (2^opcode_width_bits).
    pub opcode_space: usize,
    /// Number of opcodes actually assigned in the registry.
    pub assigned_opcodes: usize,
    /// Number of general-purpose registers.
    pub register_count: usize,
    /// Architecture identifier (e.g., "JX" for JXCL).
    pub architecture_id: u16,
    /// Binary format version.
    pub binary_version: u16,
    /// Register width (should match word_width_bits).
    pub register_width_bits: u32,
}

impl IsaMetadata {
    /// Generates the authoritative ISA metadata from the live
    /// architecture constants and opcode registry.
    pub fn describe() -> Self {
        IsaMetadata {
            word_width_bits: WORD_BITS,
            address_width_bits: ADDRESS_WIDTH_BITS,
            opcode_width_bits: OPCODE_BITS,
            opcode_space: OPCODE_SPACE,
            assigned_opcodes: assigned_opcode_count(),
            register_count: REGISTER_COUNT,
            architecture_id: ARCHITECTURE_ID,
            binary_version: BINARY_VERSION,
            register_width_bits: REGISTER_WIDTH_BITS,
        }
    }

    /// Check internal consistency of the metadata (word width and
    /// address width match, opcode space is correct, etc.).
    pub fn validate(&self) -> Result<(), String> {
        // Word and address widths should be 64 bits for JXCL
        if self.word_width_bits != 64 {
            return Err(format!(
                "word width mismatch: expected 64, got {}",
                self.word_width_bits
            ));
        }
        if self.address_width_bits != 64 {
            return Err(format!(
                "address width mismatch: expected 64, got {}",
                self.address_width_bits
            ));
        }

        // Opcode space should be 2^opcode_width_bits
        let expected_space = 1usize << self.opcode_width_bits;
        if self.opcode_space != expected_space {
            return Err(format!(
                "opcode space mismatch: expected {}, got {}",
                expected_space, self.opcode_space
            ));
        }

        // Assigned opcodes should not exceed the opcode space
        if self.assigned_opcodes > self.opcode_space {
            return Err(format!(
                "assigned opcodes ({}) exceeds opcode space ({})",
                self.assigned_opcodes, self.opcode_space
            ));
        }

        // Register count should match the constant
        if self.register_count != REGISTER_COUNT {
            return Err(format!(
                "register count mismatch: expected {}, got {}",
                REGISTER_COUNT, self.register_count
            ));
        }

        // Register width should match word width
        if self.register_width_bits != self.word_width_bits {
            return Err(format!(
                "register width {} does not match word width {}",
                self.register_width_bits, self.word_width_bits
            ));
        }

        Ok(())
    }

    /// Get a human-readable summary of the ISA.
    pub fn summary(&self) -> String {
        format!(
            "JXCL ISA: {}-bit words, {} registers, {} assigned opcodes (out of {}), version {}.{}",
            self.word_width_bits,
            self.register_count,
            self.assigned_opcodes,
            self.opcode_space,
            self.architecture_id >> 8,
            self.architecture_id & 0xFF
        )
    }
}

impl std::fmt::Display for IsaMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ISA Metadata:\n  Word width: {} bits\n  Address width: {} bits\n  \
             Opcode width: {} bits\n  Opcode space: {} (assigned: {})\n  \
             Register count: {}\n  Architecture ID: {:#06x}\n  Binary version: {}",
            self.word_width_bits,
            self.address_width_bits,
            self.opcode_width_bits,
            self.opcode_space,
            self.assigned_opcodes,
            self.register_count,
            self.architecture_id,
            self.binary_version
        )
    }
}

/// Convenience function that generates and returns the authoritative ISA
/// metadata. Equivalent to `IsaMetadata::describe()`.
pub fn describe() -> IsaMetadata {
    IsaMetadata::describe()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_describes_live_isa() {
        let meta = IsaMetadata::describe();
        assert_eq!(meta.word_width_bits, 64);
        assert_eq!(meta.address_width_bits, 64);
        assert_eq!(meta.opcode_width_bits, 8);
        assert_eq!(meta.register_count, 32);
    }

    #[test]
    fn metadata_is_self_consistent() {
        let meta = IsaMetadata::describe();
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn opcode_space_is_correct() {
        let meta = IsaMetadata::describe();
        assert_eq!(meta.opcode_space, 256); // 2^8
        assert!(meta.assigned_opcodes <= meta.opcode_space);
    }

    #[test]
    fn assigned_opcodes_matches_registry() {
        let meta = IsaMetadata::describe();
        let registry_count = all_defs().len();
        assert_eq!(meta.assigned_opcodes, registry_count);
    }

    #[test]
    fn metadata_validation_catches_inconsistencies() {
        let mut bad_meta = IsaMetadata::describe();
        bad_meta.word_width_bits = 32; // Inconsistent!
        assert!(bad_meta.validate().is_err());

        let mut bad_meta2 = IsaMetadata::describe();
        bad_meta2.assigned_opcodes = bad_meta2.opcode_space + 1; // Exceeds space
        assert!(bad_meta2.validate().is_err());
    }

    #[test]
    fn summary_formatting() {
        let meta = IsaMetadata::describe();
        let summary = meta.summary();
        assert!(summary.contains("64-bit"));
        assert!(summary.contains("32 registers"));
        assert!(summary.contains("opcodes"));
    }

    #[test]
    fn display_formatting() {
        let meta = IsaMetadata::describe();
        let display = meta.to_string();
        assert!(display.contains("ISA Metadata"));
        assert!(display.contains("Word width"));
        assert!(display.contains("Register count"));
    }

    #[test]
    fn boundary_register_count() {
        let meta = IsaMetadata::describe();
        assert!(meta.register_count > 0);
        assert_eq!(meta.register_count, REGISTER_COUNT);
    }

    #[test]
    fn architecture_id_is_nonzero() {
        let meta = IsaMetadata::describe();
        assert_ne!(meta.architecture_id, 0);
        assert_eq!(meta.architecture_id, ARCHITECTURE_ID);
    }
}
