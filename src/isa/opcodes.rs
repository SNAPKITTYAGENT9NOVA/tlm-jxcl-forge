//! The opcode registry (spec §19): the single authoritative source of
//! truth for every JXCL instruction's mnemonic, opcode byte, wire format
//! and flag effects. The encoder, decoder, assembler, disassembler and
//! execution engine all derive their behavior from this table; none of
//! them hard-code opcode numbers or formats independently (spec §19:
//! "Avoid duplicated opcode definitions across the assembler, decoder,
//! and executor.").

use crate::isa::flags::FlagEffect;
use crate::isa::operand::Format;

/// Every JXCL mnemonic, formally enumerated (spec §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mnemonic {
    // Data movement
    Mov, Movi, Load, Store, Push, Pop, Lea,
    // Integer arithmetic
    Add, Sub, Adc, Sbc, Mul, Mulh, Div, Rem, Neg, Inc, Dec,
    // Logical
    And, Or, Xor, Not, Nand, Nor, Xor3,
    // Shift / rotate
    Shl, Shr, Sar, Rol, Ror,
    // Comparison
    Cmp, Test,
    // Control flow
    Jmp, Call, Ret, Jz, Jnz, Jc, Jnc, Jl, Jle, Jg, Jge,
    // System
    Nop, Halt, Trap, Sys,
    // Memory / atomic
    Cas, Xchg, Fence,
}

impl Mnemonic {
    /// Canonical uppercase assembly mnemonic text (spec §16, §17: the
    /// disassembler's canonical output must round-trip through the
    /// assembler byte-for-byte, so this text is authoritative).
    pub const fn text(self) -> &'static str {
        use Mnemonic::*;
        match self {
            Mov => "MOV", Movi => "MOVI", Load => "LOAD", Store => "STORE",
            Push => "PUSH", Pop => "POP", Lea => "LEA",
            Add => "ADD", Sub => "SUB", Adc => "ADC", Sbc => "SBC",
            Mul => "MUL", Mulh => "MULH", Div => "DIV", Rem => "REM",
            Neg => "NEG", Inc => "INC", Dec => "DEC",
            And => "AND", Or => "OR", Xor => "XOR", Not => "NOT",
            Nand => "NAND", Nor => "NOR", Xor3 => "XOR3",
            Shl => "SHL", Shr => "SHR", Sar => "SAR", Rol => "ROL", Ror => "ROR",
            Cmp => "CMP", Test => "TEST",
            Jmp => "JMP", Call => "CALL", Ret => "RET",
            Jz => "JZ", Jnz => "JNZ", Jc => "JC", Jnc => "JNC",
            Jl => "JL", Jle => "JLE", Jg => "JG", Jge => "JGE",
            Nop => "NOP", Halt => "HALT", Trap => "TRAP", Sys => "SYS",
            Cas => "CAS", Xchg => "XCHG", Fence => "FENCE",
        }
    }

    pub fn from_text(s: &str) -> Option<Mnemonic> {
        ALL.iter().copied().find(|m| m.text() == s)
    }
}

/// One authoritative instruction definition (spec §19: mnemonic, opcode,
/// operand schema/format, semantic operation is implemented in
/// `execution.rs` keyed by this same mnemonic, flag effects, doc id).
#[derive(Debug, Clone, Copy)]
pub struct InstructionDef {
    pub mnemonic: Mnemonic,
    pub opcode: u8,
    pub format: Format,
    pub flags: FlagEffect,
    /// Stable identifier used to cross-reference generated documentation
    /// (spec §19 "documentation identifier", spec §43).
    pub doc_id: &'static str,
}

macro_rules! registry {
    ( $( $mnemonic:ident = $opcode:expr, $format:ident, $flags:expr, $doc:expr ; )* ) => {
        pub const ALL: &[Mnemonic] = &[ $( Mnemonic::$mnemonic ),* ];

        const DEFS: &[InstructionDef] = &[
            $(
                InstructionDef {
                    mnemonic: Mnemonic::$mnemonic,
                    opcode: $opcode,
                    format: Format::$format,
                    flags: $flags,
                    doc_id: $doc,
                },
            )*
        ];
    };
}

// The authoritative opcode table. Opcode numbers are frozen once assigned;
// see docs/ISA_SPEC.md "Opcode Map" for the generated human-readable form.
registry! {
    // Data movement — 0x00..0x0F
    Nop   = 0x00, None,   FlagEffect::NONE,  "sys.nop";
    Mov   = 0x01, RR,     FlagEffect::NONE,  "data.mov";
    Movi  = 0x02, RImm64, FlagEffect::NONE,  "data.movi";
    Load  = 0x03, RMem,   FlagEffect::NONE,  "data.load";
    Store = 0x04, MemR,   FlagEffect::NONE,  "data.store";
    Push  = 0x05, R,      FlagEffect::NONE,  "data.push";
    Pop   = 0x06, R,      FlagEffect::NONE,  "data.pop";
    Lea   = 0x07, RMem,   FlagEffect::NONE,  "data.lea";

    // Integer arithmetic — 0x10..0x1F
    Add = 0x10, RR, FlagEffect::ZNCV_FULL, "alu.add";
    Sub = 0x11, RR, FlagEffect::ZNCV_FULL, "alu.sub";
    Adc = 0x12, RR, FlagEffect::ZNCV_FULL, "alu.adc";
    Sbc = 0x13, RR, FlagEffect::ZNCV_FULL, "alu.sbc";
    Mul   = 0x14, RR, FlagEffect::ZNCV_FULL, "alu.mul";
    Mulh  = 0x15, RR, FlagEffect::ZN,       "alu.mulh";
    Div   = 0x16, RR, FlagEffect::ZN,       "alu.div";
    Rem   = 0x17, RR, FlagEffect::ZN,       "alu.rem";
    Neg   = 0x18, R,  FlagEffect::ZNCV_FULL, "alu.neg";
    Inc   = 0x19, R,  FlagEffect::ZNV,      "alu.inc";
    Dec   = 0x1A, R,  FlagEffect::ZNV,      "alu.dec";

    // Logical — 0x20..0x2F
    And  = 0x20, RR,  FlagEffect::ZN, "logic.and";
    Or   = 0x21, RR,  FlagEffect::ZN, "logic.or";
    Xor  = 0x22, RR,  FlagEffect::ZN, "logic.xor";
    Not  = 0x23, R,   FlagEffect::ZN, "logic.not";
    Nand = 0x24, RR,  FlagEffect::ZN, "logic.nand";
    Nor  = 0x25, RR,  FlagEffect::ZN, "logic.nor";
    Xor3 = 0x26, RRR, FlagEffect::ZN, "logic.xor3";

    // Shift / rotate — 0x30..0x3F
    Shl = 0x30, RR, FlagEffect::ZNC, "shift.shl";
    Shr = 0x31, RR, FlagEffect::ZNC, "shift.shr";
    Sar = 0x32, RR, FlagEffect::ZNC, "shift.sar";
    Rol = 0x33, RR, FlagEffect::ZNC, "shift.rol";
    Ror = 0x34, RR, FlagEffect::ZNC, "shift.ror";

    // Comparison — 0x40..0x4F
    Cmp  = 0x40, RR, FlagEffect::ZNCV_FULL, "cmp.cmp";
    Test = 0x41, RR, FlagEffect::ZN,        "cmp.test";

    // Control flow — 0x50..0x5F
    Jmp  = 0x50, BranchImm32, FlagEffect::NONE, "cf.jmp";
    Call = 0x51, BranchImm32, FlagEffect::NONE, "cf.call";
    Ret  = 0x52, None,        FlagEffect::NONE, "cf.ret";
    Jz   = 0x53, BranchImm32, FlagEffect::NONE, "cf.jz";
    Jnz  = 0x54, BranchImm32, FlagEffect::NONE, "cf.jnz";
    Jc   = 0x55, BranchImm32, FlagEffect::NONE, "cf.jc";
    Jnc  = 0x56, BranchImm32, FlagEffect::NONE, "cf.jnc";
    Jl   = 0x57, BranchImm32, FlagEffect::NONE, "cf.jl";
    Jle  = 0x58, BranchImm32, FlagEffect::NONE, "cf.jle";
    Jg   = 0x59, BranchImm32, FlagEffect::NONE, "cf.jg";
    Jge  = 0x5A, BranchImm32, FlagEffect::NONE, "cf.jge";

    // System — 0x60..0x6F
    Halt = 0x60, None,  FlagEffect::NONE, "sys.halt";
    Trap = 0x61, Imm16, FlagEffect::NONE, "sys.trap";
    Sys  = 0x62, Imm16, FlagEffect::NONE, "sys.sys";

    // Memory / atomic — 0x70..0x7F
    Cas   = 0x70, Cas,  FlagEffect::ZN, "atomic.cas";
    Xchg  = 0x71, RMem, FlagEffect::NONE, "atomic.xchg";
    Fence = 0x72, None, FlagEffect::NONE, "atomic.fence";
}

/// Look up an instruction definition by its decoded opcode byte.
pub fn lookup_opcode(opcode: u8) -> Option<&'static InstructionDef> {
    DEFS.iter().find(|d| d.opcode == opcode)
}

/// Look up an instruction definition by mnemonic.
pub fn lookup_mnemonic(mnemonic: Mnemonic) -> &'static InstructionDef {
    DEFS.iter()
        .find(|d| d.mnemonic == mnemonic)
        .expect("every Mnemonic variant has exactly one registry entry")
}

/// Iterate the full registry, e.g. for documentation generation (spec §43)
/// or exhaustive property tests (spec §32).
pub fn all_defs() -> &'static [InstructionDef] {
    DEFS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_mnemonic_has_exactly_one_entry() {
        let mut seen = HashSet::new();
        for m in ALL {
            assert!(seen.insert(*m), "duplicate registry entry for {:?}", m);
        }
        assert_eq!(seen.len(), ALL.len());
    }

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
            let found2 = lookup_mnemonic(d.mnemonic);
            assert_eq!(found2.opcode, d.opcode);
        }
    }

    #[test]
    fn mnemonic_text_roundtrips() {
        for m in ALL {
            let text = m.text();
            assert_eq!(Mnemonic::from_text(text), Some(*m));
        }
    }
}
