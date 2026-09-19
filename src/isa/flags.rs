//! Flag subsystem (spec §6): explicit Z/N/C/V flags and per-instruction
//! flag-effect declarations.

/// Bit positions of each architectural flag within the raw `flags` word.
pub const Z_BIT: u64 = 1 << 0;
pub const N_BIT: u64 = 1 << 1;
pub const C_BIT: u64 = 1 << 2;
pub const V_BIT: u64 = 1 << 3;

/// Decoded view of the flags word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub z: bool,
    pub n: bool,
    pub c: bool,
    pub v: bool,
}

impl Flags {
    pub fn from_bits(bits: u64) -> Self {
        Flags {
            z: bits & Z_BIT != 0,
            n: bits & N_BIT != 0,
            c: bits & C_BIT != 0,
            v: bits & V_BIT != 0,
        }
    }

    pub fn to_bits(self) -> u64 {
        (self.z as u64 * Z_BIT)
            | (self.n as u64 * N_BIT)
            | (self.c as u64 * C_BIT)
            | (self.v as u64 * V_BIT)
    }

    /// Compute Z/N from a 64-bit result, leaving C/V untouched (caller sets them).
    pub fn zn_of(result: u64) -> (bool, bool) {
        (result == 0, (result as i64) < 0)
    }
}

/// Declares exactly which flags an instruction is architecturally permitted
/// to modify (spec §6). Instructions never touch flags outside their mask;
/// this is enforced by the ALU/execution layer, not left implicit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlagEffect {
    pub z: bool,
    pub n: bool,
    pub c: bool,
    pub v: bool,
}

impl FlagEffect {
    pub const NONE: FlagEffect = FlagEffect { z: false, n: false, c: false, v: false };
    pub const ZN: FlagEffect = FlagEffect { z: true, n: true, c: false, v: false };
    pub const ZNC: FlagEffect = FlagEffect { z: true, n: true, c: true, v: false };
    /// Z, N, V — used by INC/DEC, which by architectural decision leave C
    /// unchanged (spec §6 example table doesn't cover INC/DEC directly;
    /// documented in docs/ISA_SPEC.md "Flag Effects").
    pub const ZNV: FlagEffect = FlagEffect { z: true, n: true, c: false, v: true };
    pub const ZNCV_FULL: FlagEffect = FlagEffect { z: true, n: true, c: true, v: true };
}

/// Merge a freshly computed `Flags` value into the current flags word,
/// updating only the bits declared by `effect` and leaving the rest
/// architecturally unchanged.
pub fn apply_effect(current_bits: u64, effect: FlagEffect, computed: Flags) -> u64 {
    let mut bits = current_bits;
    if effect.z {
        bits = set_bit(bits, Z_BIT, computed.z);
    }
    if effect.n {
        bits = set_bit(bits, N_BIT, computed.n);
    }
    if effect.c {
        bits = set_bit(bits, C_BIT, computed.c);
    }
    if effect.v {
        bits = set_bit(bits, V_BIT, computed.v);
    }
    bits
}

fn set_bit(bits: u64, mask: u64, value: bool) -> u64 {
    if value {
        bits | mask
    } else {
        bits & !mask
    }
}
