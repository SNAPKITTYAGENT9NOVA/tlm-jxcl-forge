//! The flags register: zero/carry/overflow/negative bit semantics.
//!
//! Extracted verbatim (in behavior) from `jxcl/src/isa/flags.rs`. The
//! pre-expansion source operates on raw `u64` bit words directly; this
//! crate keeps that exact bit layout and merge semantics (spec §6) but
//! expresses the raw word as `jxcl-types::Word` at the crate boundary
//! instead of a bare `u64`, so every ISA crate agrees on one
//! representation for architectural words.
//!
//! Owns: the `Flags` type and its bit-level get/set semantics, plus
//! `FlagEffect` (which flags a given instruction is permitted to touch)
//! and `apply_effect` (the masked merge of a freshly computed `Flags`
//! into a live flags word).
#![forbid(unsafe_code)]

use jxcl_types::Word;

/// Bit positions of each architectural flag within the raw flags word.
pub const Z_BIT: u64 = 1 << 0;
pub const N_BIT: u64 = 1 << 1;
pub const C_BIT: u64 = 1 << 2;
pub const V_BIT: u64 = 1 << 3;

/// Decoded view of the flags word (spec §6): Zero, Negative, Carry,
/// oVerflow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub z: bool,
    pub n: bool,
    pub c: bool,
    pub v: bool,
}

impl Flags {
    /// Decode a `Flags` value from the low 4 bits of a raw flags word.
    pub fn from_bits(bits: u64) -> Self {
        Flags {
            z: bits & Z_BIT != 0,
            n: bits & N_BIT != 0,
            c: bits & C_BIT != 0,
            v: bits & V_BIT != 0,
        }
    }

    /// Encode this `Flags` value back into a raw flags word (bits above
    /// bit 3 are always zero).
    pub fn to_bits(self) -> u64 {
        (self.z as u64 * Z_BIT)
            | (self.n as u64 * N_BIT)
            | (self.c as u64 * C_BIT)
            | (self.v as u64 * V_BIT)
    }

    /// Decode a `Flags` value from a raw architectural [`Word`].
    pub fn from_word(word: Word) -> Self {
        Self::from_bits(word.get())
    }

    /// Encode this `Flags` value as a raw architectural [`Word`].
    pub fn to_word(self) -> Word {
        Word::new(self.to_bits())
    }

    /// Compute Z/N from a 64-bit result, leaving C/V untouched (the
    /// caller -- the ALU -- computes those from the operation itself).
    pub fn zn_of(result: u64) -> (bool, bool) {
        (result == 0, (result as i64) < 0)
    }
}

/// Declares exactly which flags an instruction is architecturally
/// permitted to modify (spec §6). Instructions never touch flags outside
/// their mask; this is enforced by the ALU/execution layer, not left
/// implicit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlagEffect {
    pub z: bool,
    pub n: bool,
    pub c: bool,
    pub v: bool,
}

impl FlagEffect {
    pub const NONE: FlagEffect = FlagEffect {
        z: false,
        n: false,
        c: false,
        v: false,
    };
    pub const ZN: FlagEffect = FlagEffect {
        z: true,
        n: true,
        c: false,
        v: false,
    };
    pub const ZNC: FlagEffect = FlagEffect {
        z: true,
        n: true,
        c: true,
        v: false,
    };
    /// Z, N, V — used by INC/DEC, which by architectural decision leave C
    /// unchanged (documented in `docs/ISA_SPEC.md` "Flag Effects").
    pub const ZNV: FlagEffect = FlagEffect {
        z: true,
        n: true,
        c: false,
        v: true,
    };
    pub const ZNCV_FULL: FlagEffect = FlagEffect {
        z: true,
        n: true,
        c: true,
        v: true,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bits_decodes_each_flag_independently() {
        assert_eq!(Flags::from_bits(Z_BIT).z, true);
        assert_eq!(Flags::from_bits(N_BIT).n, true);
        assert_eq!(Flags::from_bits(C_BIT).c, true);
        assert_eq!(Flags::from_bits(V_BIT).v, true);
        assert_eq!(Flags::from_bits(0), Flags::default());
    }

    #[test]
    fn to_bits_is_the_inverse_of_from_bits() {
        for bits in 0u64..16 {
            assert_eq!(Flags::from_bits(bits).to_bits(), bits);
        }
    }

    #[test]
    fn word_roundtrip_matches_bits_roundtrip() {
        let f = Flags {
            z: true,
            n: false,
            c: true,
            v: false,
        };
        assert_eq!(Flags::from_word(f.to_word()), f);
        assert_eq!(f.to_word(), Word::new(f.to_bits()));
    }

    #[test]
    fn zn_of_zero_result() {
        assert_eq!(Flags::zn_of(0), (true, false));
    }

    #[test]
    fn zn_of_negative_result() {
        // High bit set => negative in two's complement.
        assert_eq!(Flags::zn_of(u64::MAX), (false, true));
    }

    #[test]
    fn zn_of_positive_nonzero_result() {
        assert_eq!(Flags::zn_of(7), (false, false));
    }

    #[test]
    fn apply_effect_none_changes_nothing() {
        let current = Z_BIT | C_BIT;
        let computed = Flags {
            z: false,
            n: true,
            c: false,
            v: true,
        };
        assert_eq!(apply_effect(current, FlagEffect::NONE, computed), current);
    }

    #[test]
    fn apply_effect_zn_leaves_c_and_v_untouched() {
        let current = C_BIT | V_BIT;
        let computed = Flags {
            z: true,
            n: true,
            c: false,
            v: false,
        };
        let result = apply_effect(current, FlagEffect::ZN, computed);
        let decoded = Flags::from_bits(result);
        assert!(decoded.z && decoded.n);
        // C and V untouched by ZN, so they retain their prior (set) value.
        assert!(decoded.c && decoded.v);
    }

    #[test]
    fn apply_effect_full_overwrites_all_four() {
        let current = Z_BIT | N_BIT | C_BIT | V_BIT;
        let computed = Flags::default();
        let result = apply_effect(current, FlagEffect::ZNCV_FULL, computed);
        assert_eq!(result, 0);
    }

    #[test]
    fn apply_effect_znv_leaves_carry_untouched() {
        let current = C_BIT;
        let computed = Flags {
            z: false,
            n: false,
            c: false,
            v: true,
        };
        let result = apply_effect(current, FlagEffect::ZNV, computed);
        let decoded = Flags::from_bits(result);
        assert!(decoded.c, "ZNV must not clear a pre-existing carry bit");
        assert!(decoded.v);
        assert!(!decoded.z && !decoded.n);
    }
}
