//! Operand model (spec §8, §20): the wire formats an instruction may take,
//! and the decoded operand values produced by the decoder.
//!
//! Every register field occupies one whole byte (spec §4 rationale, see
//! `constants::REGISTER_FIELD_BYTES`); immediates and displacements are
//! fixed-width, little-endian, and their width is determined solely by
//! the opcode (never by a runtime flag), so instruction boundaries are
//! always deterministic (spec §4: "No instruction may have ambiguous
//! decoding.").

use crate::isa::registers::RegId;

/// The wire format of an instruction, keyed by opcode in the registry.
/// This determines the *encoded length* of the instruction independent
/// of any operand value.
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
    /// Total encoded instruction length in bytes, including the opcode byte.
    /// (Never zero — the opcode byte alone is `Format::None`'s whole
    /// encoding — so there is no meaningful `is_empty` to pair this with.)
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

/// Decoded operand values. Each variant corresponds 1:1 with a `Format`,
/// so a `DecodedInstruction`'s operands can never mismatch its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operands {
    None,
    R { rd: RegId },
    RR { rd: RegId, rs: RegId },
    RRR { rd: RegId, rs1: RegId, rs2: RegId },
    RImm64 { rd: RegId, imm: u64 },
    RMem { rd: RegId, base: RegId, disp: i32 },
    MemR { base: RegId, disp: i32, rs: RegId },
    Cas { rd: RegId, base: RegId, rs_new: RegId, disp: i32 },
    BranchImm32 { disp: i32 },
    Imm16 { imm: u16 },
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
}
