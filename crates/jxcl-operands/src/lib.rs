// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The operand/addressing-mode model (register-direct, immediate,
//! memory-indirect, etc.), extracted from `jxcl/src/isa/operand.rs`.
//!
//! Every register field occupies one whole byte (spec §4 rationale, see
//! `jxcl-constants::REGISTER_FIELD_BYTES`); immediates and displacements
//! are fixed-width, little-endian, and their width is determined solely
//! by the opcode (never by a runtime flag), so instruction boundaries
//! are always deterministic (spec §4: "No instruction may have
//! ambiguous decoding.").
//!
//! ## Deviation from `docs/crates.toml`
//!
//! The registry names this crate's items `Operand`/`AddressingMode`. The
//! real pre-expansion source (and every downstream extraction crate in
//! this batch: `jxcl-instructions`, `jxcl-encoding`, `jxcl-decoding`)
//! names the wire-format-tag type `Format` and the decoded-values type
//! `Operands`. This crate keeps those real names as the primary API and
//! additionally exposes `AddressingMode`/`Operand` as aliases, matching
//! the registry's literal `public_api` for a reader who starts there.
//! Register fields use `jxcl-registers::Register` in place of the
//! pre-expansion bare `RegId = u8`.
#![forbid(unsafe_code)]

/// Re-exported (not just imported) so that downstream crates whose
/// `docs/crates.toml` dependency list includes `jxcl-operands` but not
/// `jxcl-registers` directly (`jxcl-instructions`, and transitively
/// `jxcl-encoding`/`jxcl-decoding` via `jxcl-instructions`) can still
/// name register-operand values as `jxcl_operands::Register` without an
/// otherwise-redundant direct dependency.
pub use jxcl_registers::Register;

/// The wire format of an instruction, keyed by opcode in the registry.
/// This determines the *encoded length* of the instruction independent
/// of any operand value.
///
/// Alias matching `docs/crates.toml`'s `public_api` naming
/// (`AddressingMode`); see the module-level "Deviation" note.
pub type AddressingMode = Format;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// opcode only.
    None,
    /// opcode + rd.
    R,
    /// opcode + rd + rs.
    RR,
    /// opcode + rd + rs1 + rs2.
    RRR,
    /// opcode + rd + imm64 (little-endian).
    RImm64,
    /// opcode + rd + base + disp32 (little-endian, signed). Effective
    /// address = R[base] + sign_extend(disp32).
    RMem,
    /// opcode + base + disp32 + rs. Effective address as above.
    MemR,
    /// opcode + rd + base + rs_new + disp32 (CAS's unique layout, spec §25).
    Cas,
    /// opcode + imm32 (little-endian, signed), PC-relative branch/call target.
    BranchImm32,
    /// opcode + imm16 (little-endian), SYS/TRAP identifier.
    Imm16,
}

impl Format {
    /// Total encoded instruction length in bytes, including the opcode
    /// byte. (Never zero -- the opcode byte alone is `Format::None`'s
    /// whole encoding -- so there is no meaningful `is_empty` to pair
    /// this with.)
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(self) -> usize {
        match self {
            Format::None => 1,
            Format::R => 2,
            Format::RR => 3,
            Format::RRR => 4,
            Format::RImm64 => 1 + 1 + 8,
            Format::RMem => 1 + 1 + 1 + 4,
            Format::MemR => 1 + 1 + 4 + 1,
            Format::Cas => 1 + 1 + 1 + 1 + 4,
            Format::BranchImm32 => 1 + 4,
            Format::Imm16 => 1 + 2,
        }
    }
}

/// Alias matching `docs/crates.toml`'s `public_api` naming (`Operand`);
/// see the module-level "Deviation" note. The real name, `Operands`, is
/// kept as the primary type since it is what every downstream extraction
/// crate in this batch is written against.
pub type Operand = Operands;

/// Decoded operand values. Each variant corresponds 1:1 with a `Format`,
/// so an instruction's operands can never mismatch its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operands {
    None,
    R {
        rd: Register,
    },
    RR {
        rd: Register,
        rs: Register,
    },
    RRR {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    RImm64 {
        rd: Register,
        imm: u64,
    },
    RMem {
        rd: Register,
        base: Register,
        disp: i32,
    },
    MemR {
        base: Register,
        disp: i32,
        rs: Register,
    },
    Cas {
        rd: Register,
        base: Register,
        rs_new: Register,
        disp: i32,
    },
    BranchImm32 {
        disp: i32,
    },
    Imm16 {
        imm: u16,
    },
}

impl Operands {
    pub const fn format(&self) -> Format {
        match self {
            Operands::None => Format::None,
            Operands::R { .. } => Format::R,
            Operands::RR { .. } => Format::RR,
            Operands::RRR { .. } => Format::RRR,
            Operands::RImm64 { .. } => Format::RImm64,
            Operands::RMem { .. } => Format::RMem,
            Operands::MemR { .. } => Format::MemR,
            Operands::Cas { .. } => Format::Cas,
            Operands::BranchImm32 { .. } => Format::BranchImm32,
            Operands::Imm16 { .. } => Format::Imm16,
        }
    }

    /// Every register operand this instruction's operands reference, in
    /// encoding order. Used by `jxcl-instruction-validation` (and any
    /// other caller that needs to check register legality) without
    /// re-deriving the per-format field layout.
    pub fn registers(&self) -> Vec<Register> {
        match *self {
            Operands::None | Operands::BranchImm32 { .. } | Operands::Imm16 { .. } => vec![],
            Operands::R { rd } => vec![rd],
            Operands::RR { rd, rs } => vec![rd, rs],
            Operands::RRR { rd, rs1, rs2 } => vec![rd, rs1, rs2],
            Operands::RImm64 { rd, .. } => vec![rd],
            Operands::RMem { rd, base, .. } => vec![rd, base],
            Operands::MemR { base, rs, .. } => vec![base, rs],
            Operands::Cas {
                rd, base, rs_new, ..
            } => vec![rd, base, rs_new],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_len_matches_isa_spec_table() {
        assert_eq!(Format::None.len(), 1);
        assert_eq!(Format::R.len(), 2);
        assert_eq!(Format::RR.len(), 3);
        assert_eq!(Format::RRR.len(), 4);
        assert_eq!(Format::RImm64.len(), 10);
        assert_eq!(Format::RMem.len(), 7);
        assert_eq!(Format::MemR.len(), 7);
        assert_eq!(Format::Cas.len(), 8);
        assert_eq!(Format::BranchImm32.len(), 5);
        assert_eq!(Format::Imm16.len(), 3);
    }

    #[test]
    fn operands_format_matches_variant() {
        assert_eq!(Operands::None.format(), Format::None);
        assert_eq!(
            Operands::R {
                rd: Register::new(1)
            }
            .format(),
            Format::R
        );
        assert_eq!(
            Operands::RImm64 {
                rd: Register::new(1),
                imm: 0
            }
            .format(),
            Format::RImm64
        );
        assert_eq!(
            Operands::BranchImm32 { disp: 4 }.format(),
            Format::BranchImm32
        );
    }

    #[test]
    fn registers_extracts_every_register_field_in_order() {
        let ops = Operands::Cas {
            rd: Register::new(1),
            base: Register::new(2),
            rs_new: Register::new(3),
            disp: 0,
        };
        assert_eq!(
            ops.registers(),
            vec![Register::new(1), Register::new(2), Register::new(3)]
        );
    }

    #[test]
    fn registers_empty_for_immediate_and_branch_forms() {
        assert_eq!(Operands::None.registers(), vec![]);
        assert_eq!(Operands::BranchImm32 { disp: 1 }.registers(), vec![]);
        assert_eq!(Operands::Imm16 { imm: 1 }.registers(), vec![]);
    }

    #[test]
    fn addressing_mode_and_operand_aliases_are_the_real_types() {
        let mode: AddressingMode = Format::RR;
        assert_eq!(mode.len(), 3);
        let operand: Operand = Operands::None;
        assert_eq!(operand.format(), Format::None);
    }
}
