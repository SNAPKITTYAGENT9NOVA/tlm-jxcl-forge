// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The `Instruction` type tying an opcode to its operands -- the
//! canonical in-memory instruction representation.
//!
//! Extracted from `jxcl/src/isa/instruction.rs` (there named
//! `DecodedInstruction`). This crate keeps the real name
//! `DecodedInstruction` as the primary type (every downstream extraction
//! crate in this batch, `jxcl-encoding`/`jxcl-decoding`, is written
//! against it) and additionally exposes `Instruction` as an alias,
//! matching `docs/crates.toml`'s literal `public_api`.
//!
//! Owns: the `Instruction`/`DecodedInstruction` struct -- the single
//! representation the encoder produces bytes from and the decoder
//! produces from bytes.
//!
//! ## Re-exports and the opcode/operand `Format` bridge
//!
//! `docs/crates.toml` gives `jxcl-encoding`/`jxcl-decoding` a dependency
//! list of `jxcl-instructions`, `jxcl-bitops`, `jxcl-endian`,
//! `jxcl-bytes`, `jxcl-errors` -- deliberately *not* `jxcl-opcodes` or
//! `jxcl-operands` directly. This crate therefore re-exports the pieces
//! of those two crates' public API that the encoder/decoder need
//! (`Mnemonic`, `Operands`/`Format`, `Register`) so they never need a
//! redundant direct dependency, and additionally provides the one piece
//! of glue logic only possible here: `jxcl-opcodes::Format` (the
//! registry's per-opcode wire-format tag) and `jxcl-operands::Format`
//! (the decoded operand values' actual shape) are two structurally
//! identical but distinct Rust types, defined in two crates that
//! deliberately don't depend on each other (see `jxcl-opcodes`'s module
//! doc for why: `jxcl-operands` depending on `jxcl-opcodes` the other
//! direction would create a cycle). [`format_matches_registry`] and
//! [`instruction_shape_for_opcode`] are the mechanical bridge between
//! them, used by the encoder to reject an operand shape that doesn't
//! match its mnemonic's registered format, and by the decoder to know
//! which operand shape to parse for a given opcode byte.
#![forbid(unsafe_code)]

pub use jxcl_opcodes::Mnemonic;
pub use jxcl_operands::{Format, Operands, Register};

/// A fully decoded (or, prior to encoding, fully specified) instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub mnemonic: Mnemonic,
    pub operands: Operands,
    /// Encoded length in bytes (redundant with `operands.format().len()`,
    /// but carried explicitly so callers never need to recompute it --
    /// spec §20's `DecodedInstruction { ... length ... }`).
    pub length: usize,
}

impl DecodedInstruction {
    pub fn new(mnemonic: Mnemonic, operands: Operands) -> Self {
        DecodedInstruction {
            mnemonic,
            operands,
            length: operands.format().len(),
        }
    }
}

/// Alias matching `docs/crates.toml`'s `public_api` naming
/// (`Instruction`); see the module-level doc comment.
pub type Instruction = DecodedInstruction;

/// A backend-agnostic tag for instruction wire-format shape: the common
/// ground between `jxcl-opcodes::Format` (the registry's per-entry wire
/// format) and `jxcl-operands::Format` (an operand set's actual shape).
/// See the module-level "Re-exports and the opcode/operand `Format`
/// bridge" doc for why two distinct types exist for the same ten shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
enum FormatShape {
    None,
    R,
    RR,
    RRR,
    RImm64,
    RMem,
    MemR,
    Cas,
    BranchImm32,
    Imm16,
}

impl From<jxcl_opcodes::Format> for FormatShape {
    fn from(f: jxcl_opcodes::Format) -> Self {
        use jxcl_opcodes::Format as OF;
        match f {
            OF::None => FormatShape::None,
            OF::R => FormatShape::R,
            OF::RR => FormatShape::RR,
            OF::RRR => FormatShape::RRR,
            OF::RImm64 => FormatShape::RImm64,
            OF::RMem => FormatShape::RMem,
            OF::MemR => FormatShape::MemR,
            OF::Cas => FormatShape::Cas,
            OF::BranchImm32 => FormatShape::BranchImm32,
            OF::Imm16 => FormatShape::Imm16,
        }
    }
}

impl From<Format> for FormatShape {
    fn from(f: Format) -> Self {
        match f {
            Format::None => FormatShape::None,
            Format::R => FormatShape::R,
            Format::RR => FormatShape::RR,
            Format::RRR => FormatShape::RRR,
            Format::RImm64 => FormatShape::RImm64,
            Format::RMem => FormatShape::RMem,
            Format::MemR => FormatShape::MemR,
            Format::Cas => FormatShape::Cas,
            Format::BranchImm32 => FormatShape::BranchImm32,
            Format::Imm16 => FormatShape::Imm16,
        }
    }
}

impl From<FormatShape> for Format {
    fn from(s: FormatShape) -> Self {
        match s {
            FormatShape::None => Format::None,
            FormatShape::R => Format::R,
            FormatShape::RR => Format::RR,
            FormatShape::RRR => Format::RRR,
            FormatShape::RImm64 => Format::RImm64,
            FormatShape::RMem => Format::RMem,
            FormatShape::MemR => Format::MemR,
            FormatShape::Cas => Format::Cas,
            FormatShape::BranchImm32 => Format::BranchImm32,
            FormatShape::Imm16 => Format::Imm16,
        }
    }
}

/// The opcode byte the registry assigns to `mnemonic` (spec §19). Used
/// by `jxcl-encoding` in place of a direct `jxcl-opcodes` dependency.
pub fn opcode_byte(mnemonic: Mnemonic) -> u8 {
    jxcl_opcodes::lookup_mnemonic(mnemonic).opcode
}

/// The mnemonic and operand-value shape registered for `opcode`, or
/// `None` if no instruction is assigned that opcode. Used by
/// `jxcl-decoding` to know both what to name the decoded instruction
/// and which operand fields to parse next, in one registry lookup.
pub fn instruction_shape_for_opcode(opcode: u8) -> Option<(Mnemonic, Format)> {
    jxcl_opcodes::lookup_opcode(opcode)
        .map(|def| (def.mnemonic, FormatShape::from(def.format).into()))
}

/// True if `instr`'s operand shape matches what the opcode registry
/// declares for its mnemonic (spec §19: "avoid duplicated opcode
/// definitions across the assembler, decoder, and executor" -- this is
/// the mechanical check that a hand-built `DecodedInstruction` cannot
/// silently drift from the registry). Used by `jxcl-encoding` before
/// writing any bytes.
pub fn format_matches_registry(instr: &DecodedInstruction) -> bool {
    let def = jxcl_opcodes::lookup_mnemonic(instr.mnemonic);
    FormatShape::from(def.format) == FormatShape::from(instr.operands.format())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_computes_length_from_operands_format() {
        let instr = DecodedInstruction::new(Mnemonic::Halt, Operands::None);
        assert_eq!(instr.length, 1);

        let instr = DecodedInstruction::new(
            Mnemonic::Movi,
            Operands::RImm64 {
                rd: jxcl_registers::Register::new(0),
                imm: 42,
            },
        );
        assert_eq!(instr.length, 10);
    }

    #[test]
    fn length_matches_operands_format_len_for_every_shape() {
        let rd = jxcl_registers::Register::new(1);
        let cases = [
            Operands::None,
            Operands::R { rd },
            Operands::RR { rd, rs: rd },
            Operands::RRR {
                rd,
                rs1: rd,
                rs2: rd,
            },
            Operands::RImm64 { rd, imm: 0 },
            Operands::RMem {
                rd,
                base: rd,
                disp: 0,
            },
            Operands::MemR {
                base: rd,
                disp: 0,
                rs: rd,
            },
            Operands::Cas {
                rd,
                base: rd,
                rs_new: rd,
                disp: 0,
            },
            Operands::BranchImm32 { disp: 0 },
            Operands::Imm16 { imm: 0 },
        ];
        for ops in cases {
            let instr = DecodedInstruction::new(Mnemonic::Nop, ops);
            assert_eq!(instr.length, ops.format().len());
        }
    }

    #[test]
    fn instruction_alias_is_the_real_type() {
        let instr: Instruction = DecodedInstruction::new(Mnemonic::Ret, Operands::None);
        assert_eq!(instr.mnemonic, Mnemonic::Ret);
    }

    #[test]
    fn instructions_carry_mnemonic_and_operands_unchanged() {
        let rd = jxcl_registers::Register::new(3);
        let ops = Operands::R { rd };
        let instr = DecodedInstruction::new(Mnemonic::Push, ops);
        assert_eq!(instr.mnemonic, Mnemonic::Push);
        assert_eq!(instr.operands, ops);
    }

    #[test]
    fn opcode_byte_matches_the_registry() {
        assert_eq!(opcode_byte(Mnemonic::Halt), 0x60);
        assert_eq!(opcode_byte(Mnemonic::Add), 0x10);
    }

    #[test]
    fn instruction_shape_for_opcode_roundtrips_with_opcode_byte() {
        let (mnemonic, format) = instruction_shape_for_opcode(0x10).unwrap();
        assert_eq!(mnemonic, Mnemonic::Add);
        assert_eq!(format, Format::RR);
    }

    #[test]
    fn instruction_shape_for_unassigned_opcode_is_none() {
        assert!(instruction_shape_for_opcode(0xFF).is_none());
    }

    #[test]
    fn format_matches_registry_accepts_correctly_shaped_instruction() {
        let rd = Register::new(1);
        let instr = DecodedInstruction::new(Mnemonic::Add, Operands::RR { rd, rs: rd });
        assert!(format_matches_registry(&instr));
    }

    #[test]
    fn format_matches_registry_rejects_wrong_shape() {
        // ADD is registered as RR, not R.
        let instr = DecodedInstruction::new(
            Mnemonic::Add,
            Operands::R {
                rd: Register::new(1),
            },
        );
        assert!(!format_matches_registry(&instr));
    }

    #[test]
    fn every_registry_entry_shape_round_trips_through_the_bridge() {
        for def in jxcl_opcodes::all_defs() {
            let (mnemonic, format) = instruction_shape_for_opcode(def.opcode).unwrap();
            assert_eq!(mnemonic, def.mnemonic);
            assert_eq!(FormatShape::from(format), FormatShape::from(def.format));
        }
    }
}
