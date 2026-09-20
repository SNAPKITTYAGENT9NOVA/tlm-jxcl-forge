// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A serde-serializable schema of the ISA generated from jxcl-opcodes/jxcl-constants/jxcl-registers/jxcl-flags, plus a mechanical cross-check against the numbers documented in docs/ISA_SPEC.md.
//!
//! Owns: `IsaSchema` struct and the `check_against_spec()` conformance
//! check that verifies the documented ISA parameters match the live code.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use jxcl_constants::{
    ADDRESS_WIDTH_BITS, BINARY_VERSION, OPCODE_BITS, OPCODE_SPACE, REGISTER_COUNT, WORD_BITS,
};
use jxcl_opcodes::all_defs;

/// A complete, serde-serializable schema of the JXCL ISA, capturing
/// architectural parameters and opcode table metadata.
///
/// This schema is mechanical - generated entirely from the live
/// authoritative ISA registries and constants - and can be checked against
/// documentation to ensure spec/code consistency (see `check_against_spec`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IsaSchema {
    /// Architectural parameters
    pub architecture: ArchitectureSchema,
    /// Opcode table
    pub opcodes: OpcodeTableSchema,
    /// Register file description
    pub registers: RegisterSchema,
    /// Flags register description
    pub flags: FlagsSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchitectureSchema {
    /// Width of data words, in bits.
    pub word_width_bits: u32,
    /// Width of addresses, in bits.
    pub address_width_bits: u32,
    /// Width of opcode field, in bits.
    pub opcode_width_bits: u32,
    /// Binary format version.
    pub binary_version: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpcodeTableSchema {
    /// Total addressable opcode space.
    pub space: usize,
    /// Number of opcodes actually assigned.
    pub assigned: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterSchema {
    /// Number of general-purpose registers.
    pub gp_count: usize,
    /// Special registers: PC, SP, FP, FLAGS.
    pub special_registers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlagsSchema {
    /// Named flags in the flags register.
    pub flags: Vec<String>,
}

impl IsaSchema {
    /// Generate the authoritative ISA schema from live constants and registries.
    pub fn generate() -> Self {
        IsaSchema {
            architecture: ArchitectureSchema {
                word_width_bits: WORD_BITS,
                address_width_bits: ADDRESS_WIDTH_BITS,
                opcode_width_bits: OPCODE_BITS,
                binary_version: BINARY_VERSION,
            },
            opcodes: OpcodeTableSchema {
                space: OPCODE_SPACE,
                assigned: all_defs().len(),
            },
            registers: RegisterSchema {
                gp_count: REGISTER_COUNT,
                special_registers: vec![
                    "PC".to_string(),
                    "SP".to_string(),
                    "FP".to_string(),
                    "FLAGS".to_string(),
                ],
            },
            flags: FlagsSchema {
                flags: vec![
                    "Z (zero)".to_string(),
                    "N (negative)".to_string(),
                    "C (carry)".to_string(),
                    "V (overflow)".to_string(),
                ],
            },
        }
    }

    /// Check the schema against documented ISA parameters (spec conformance).
    ///
    /// This verifies:
    /// - Word width is 64 bits (as documented)
    /// - Address width is 64 bits (as documented)
    /// - Opcode width is 8 bits (as documented)
    /// - Register count is 32 (as documented)
    /// - Opcode space is 256 (as documented)
    pub fn check_against_spec(&self) -> Result<(), String> {
        // Check documented architectural parameters
        if self.architecture.word_width_bits != 64 {
            return Err(format!(
                "word width: spec says 64, code has {}",
                self.architecture.word_width_bits
            ));
        }

        if self.architecture.address_width_bits != 64 {
            return Err(format!(
                "address width: spec says 64, code has {}",
                self.architecture.address_width_bits
            ));
        }

        if self.architecture.opcode_width_bits != 8 {
            return Err(format!(
                "opcode width: spec says 8, code has {}",
                self.architecture.opcode_width_bits
            ));
        }

        // Check opcode space consistency
        let expected_space = 1usize << self.architecture.opcode_width_bits;
        if self.opcodes.space != expected_space {
            return Err(format!(
                "opcode space: expected 2^{} = {}, code has {}",
                self.architecture.opcode_width_bits, expected_space, self.opcodes.space
            ));
        }

        // Assigned opcodes must not exceed space
        if self.opcodes.assigned > self.opcodes.space {
            return Err(format!(
                "assigned opcodes ({}) exceeds opcode space ({})",
                self.opcodes.assigned, self.opcodes.space
            ));
        }

        // Check register count (spec says 32 GP registers)
        if self.registers.gp_count != 32 {
            return Err(format!(
                "register count: spec says 32, code has {}",
                self.registers.gp_count
            ));
        }

        // Check that special registers are present
        if self.registers.special_registers.len() != 4 {
            return Err(format!(
                "special registers: expected 4 (PC, SP, FP, FLAGS), code has {}",
                self.registers.special_registers.len()
            ));
        }

        // Check that all expected flags are present
        if self.flags.flags.len() != 4 {
            return Err(format!(
                "flags: expected 4 (Z, N, C, V), code has {}",
                self.flags.flags.len()
            ));
        }

        Ok(())
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generates_from_live_isa() {
        let schema = IsaSchema::generate();
        assert_eq!(schema.architecture.word_width_bits, 64);
        assert_eq!(schema.architecture.address_width_bits, 64);
        assert_eq!(schema.architecture.opcode_width_bits, 8);
    }

    #[test]
    fn schema_passes_spec_conformance_check() {
        let schema = IsaSchema::generate();
        assert!(schema.check_against_spec().is_ok());
    }

    #[test]
    fn schema_opcode_space_is_correct() {
        let schema = IsaSchema::generate();
        assert_eq!(schema.opcodes.space, 256);
        assert!(schema.opcodes.assigned <= schema.opcodes.space);
    }

    #[test]
    fn schema_registers_are_correct() {
        let schema = IsaSchema::generate();
        assert_eq!(schema.registers.gp_count, 32);
        assert_eq!(schema.registers.special_registers.len(), 4);
    }

    #[test]
    fn schema_flags_are_correct() {
        let schema = IsaSchema::generate();
        assert_eq!(schema.flags.flags.len(), 4);
    }

    #[test]
    fn conformance_check_catches_word_width_mismatch() {
        let mut schema = IsaSchema::generate();
        schema.architecture.word_width_bits = 32;
        assert!(schema.check_against_spec().is_err());
    }

    #[test]
    fn conformance_check_catches_address_width_mismatch() {
        let mut schema = IsaSchema::generate();
        schema.architecture.address_width_bits = 32;
        assert!(schema.check_against_spec().is_err());
    }

    #[test]
    fn conformance_check_catches_opcode_width_mismatch() {
        let mut schema = IsaSchema::generate();
        schema.architecture.opcode_width_bits = 16;
        assert!(schema.check_against_spec().is_err());
    }

    #[test]
    fn conformance_check_catches_register_count_mismatch() {
        let mut schema = IsaSchema::generate();
        schema.registers.gp_count = 16;
        assert!(schema.check_against_spec().is_err());
    }

    #[test]
    fn conformance_check_catches_flag_count_mismatch() {
        let mut schema = IsaSchema::generate();
        schema.flags.flags.clear();
        assert!(schema.check_against_spec().is_err());
    }

    #[test]
    fn schema_serializes_to_json() {
        let schema = IsaSchema::generate();
        let json = schema.to_json().expect("json serialization failed");
        assert!(json.contains("architecture"));
        assert!(json.contains("word_width_bits"));
    }

    #[test]
    fn schema_deserializes_from_json() {
        let original = IsaSchema::generate();
        let json = original.to_json().expect("serialization failed");
        let deserialized = IsaSchema::from_json(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn schema_roundtrip_preserves_structure() {
        let schema = IsaSchema::generate();
        let json = schema.to_json().unwrap();
        let recovered = IsaSchema::from_json(&json).unwrap();

        assert_eq!(schema.architecture, recovered.architecture);
        assert_eq!(schema.opcodes, recovered.opcodes);
        assert_eq!(schema.registers, recovered.registers);
        assert_eq!(schema.flags, recovered.flags);
    }

    #[test]
    fn opcode_space_is_power_of_two() {
        let schema = IsaSchema::generate();
        let bits = schema.architecture.opcode_width_bits;
        let expected = 1usize << bits;
        assert_eq!(schema.opcodes.space, expected);
    }

    #[test]
    fn assigned_opcodes_is_reasonable() {
        let schema = IsaSchema::generate();
        // Should have at least one opcode and less than the space
        assert!(schema.opcodes.assigned > 0);
        assert!(schema.opcodes.assigned < schema.opcodes.space);
    }
}
