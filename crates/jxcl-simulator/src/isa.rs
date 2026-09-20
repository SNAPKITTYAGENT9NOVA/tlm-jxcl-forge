// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A real, deliberately partial ISA table: mnemonics, opcodes, and wire
//! formats for the instruction subset this crate's embedded fetch/decode/
//! execute loop actually implements.
//!
//! Every opcode byte and operand layout below is copied verbatim from
//! `crates/jxcl/src/isa/opcodes.rs` (the ground-truth registry) rather
//! than invented -- see this module's doc comment in `lib.rs` for why a
//! subset, not the full 47-mnemonic registry, is implemented here.

/// Every mnemonic this embedded machine can execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    Nop,
    Mov,
    Movi,
    Load,
    Store,
    Push,
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Inc,
    Dec,
    And,
    Or,
    Xor,
    Not,
    Shl,
    Shr,
    Cmp,
    Jmp,
    Call,
    Ret,
    Jz,
    Jnz,
    Jl,
    Jle,
    Jg,
    Jge,
    Halt,
}

/// Wire format, matching `crates/jxcl`'s `Format` enum one-for-one for
/// every variant this crate uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// opcode only.
    None,
    /// opcode + rd.
    R,
    /// opcode + rd + rs.
    RR,
    /// opcode + rd + imm64 (little-endian).
    RImm64,
    /// opcode + rd + base + disp32 (little-endian, signed).
    RMem,
    /// opcode + base + disp32 + rs.
    MemR,
    /// opcode + imm32 (little-endian, signed), PC-relative.
    BranchImm32,
}

impl Format {
    pub const fn len(self) -> usize {
        match self {
            Format::None => 1,
            Format::R => 2,
            Format::RR => 3,
            Format::RImm64 => 1 + 1 + 8,
            Format::RMem => 1 + 1 + 1 + 4,
            Format::MemR => 1 + 1 + 4 + 1,
            Format::BranchImm32 => 1 + 4,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InstructionDef {
    pub mnemonic: Mnemonic,
    pub opcode: u8,
    pub format: Format,
}

/// The opcode registry actually implemented by this crate's `exec`
/// module. Opcode numbers match `crates/jxcl`'s real registry exactly
/// for every mnemonic present here, so hand-encoded byte streams for
/// these mnemonics are valid real JXCL bytecode, not a look-alike
/// format.
pub const DEFS: &[InstructionDef] = &[
    InstructionDef {
        mnemonic: Mnemonic::Nop,
        opcode: 0x00,
        format: Format::None,
    },
    InstructionDef {
        mnemonic: Mnemonic::Mov,
        opcode: 0x01,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Movi,
        opcode: 0x02,
        format: Format::RImm64,
    },
    InstructionDef {
        mnemonic: Mnemonic::Load,
        opcode: 0x03,
        format: Format::RMem,
    },
    InstructionDef {
        mnemonic: Mnemonic::Store,
        opcode: 0x04,
        format: Format::MemR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Push,
        opcode: 0x05,
        format: Format::R,
    },
    InstructionDef {
        mnemonic: Mnemonic::Pop,
        opcode: 0x06,
        format: Format::R,
    },
    InstructionDef {
        mnemonic: Mnemonic::Add,
        opcode: 0x10,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Sub,
        opcode: 0x11,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Mul,
        opcode: 0x14,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Div,
        opcode: 0x16,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Inc,
        opcode: 0x19,
        format: Format::R,
    },
    InstructionDef {
        mnemonic: Mnemonic::Dec,
        opcode: 0x1A,
        format: Format::R,
    },
    InstructionDef {
        mnemonic: Mnemonic::And,
        opcode: 0x20,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Or,
        opcode: 0x21,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Xor,
        opcode: 0x22,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Not,
        opcode: 0x23,
        format: Format::R,
    },
    InstructionDef {
        mnemonic: Mnemonic::Shl,
        opcode: 0x30,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Shr,
        opcode: 0x31,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Cmp,
        opcode: 0x40,
        format: Format::RR,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jmp,
        opcode: 0x50,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Call,
        opcode: 0x51,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Ret,
        opcode: 0x52,
        format: Format::None,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jz,
        opcode: 0x53,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jnz,
        opcode: 0x54,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jl,
        opcode: 0x57,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jle,
        opcode: 0x58,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jg,
        opcode: 0x59,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Jge,
        opcode: 0x5A,
        format: Format::BranchImm32,
    },
    InstructionDef {
        mnemonic: Mnemonic::Halt,
        opcode: 0x60,
        format: Format::None,
    },
];

pub fn lookup_opcode(opcode: u8) -> Option<&'static InstructionDef> {
    DEFS.iter().find(|d| d.opcode == opcode)
}

pub const NUM_GP_REGISTERS: usize = 32;
pub const DEFAULT_EXECUTION_LIMIT: u64 = 1_000_000;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_opcodes() {
        let mut seen = HashSet::new();
        for d in DEFS {
            assert!(seen.insert(d.opcode), "duplicate opcode {:#04x}", d.opcode);
        }
    }

    #[test]
    fn lookup_roundtrips() {
        for d in DEFS {
            let found = lookup_opcode(d.opcode).unwrap();
            assert_eq!(found.mnemonic, d.mnemonic);
        }
    }
}
