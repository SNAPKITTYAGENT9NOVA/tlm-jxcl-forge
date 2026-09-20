// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Pure 64-bit ALU ops for this crate's embedded execution loop, ported
//! from `crates/jxcl/src/alu.rs`'s carry/overflow conventions (x86-style
//! carry, signed overflow for V) for the operations `exec` actually
//! dispatches.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flags {
    pub z: bool,
    pub n: bool,
    pub c: bool,
    pub v: bool,
}

impl Flags {
    pub const Z_BIT: u64 = 1 << 0;
    pub const N_BIT: u64 = 1 << 1;
    pub const C_BIT: u64 = 1 << 2;
    pub const V_BIT: u64 = 1 << 3;

    pub fn to_bits(self) -> u64 {
        (self.z as u64 * Self::Z_BIT)
            | (self.n as u64 * Self::N_BIT)
            | (self.c as u64 * Self::C_BIT)
            | (self.v as u64 * Self::V_BIT)
    }

    pub fn from_bits(bits: u64) -> Self {
        Flags {
            z: bits & Self::Z_BIT != 0,
            n: bits & Self::N_BIT != 0,
            c: bits & Self::C_BIT != 0,
            v: bits & Self::V_BIT != 0,
        }
    }
}

fn zn(value: u64) -> (bool, bool) {
    (value == 0, (value as i64) < 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult {
    pub value: u64,
    pub flags: Flags,
}

pub fn add(a: u64, b: u64) -> AluResult {
    let (value, carry) = a.overflowing_add(b);
    let (_, v) = (a as i64).overflowing_add(b as i64);
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: carry, v },
    }
}

pub fn sub(a: u64, b: u64) -> AluResult {
    let value = a.wrapping_sub(b);
    let borrow = a < b;
    let (_, v) = (a as i64).overflowing_sub(b as i64);
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: borrow, v },
    }
}

/// Same result as [`sub`] but never written back -- used by `CMP`.
pub fn cmp(a: u64, b: u64) -> AluResult {
    sub(a, b)
}

pub fn mul(a: u64, b: u64) -> AluResult {
    let (value, carry) = a.overflowing_mul(b);
    let (_, v) = (a as i64).overflowing_mul(b as i64);
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: carry, v },
    }
}

/// Unsigned division. Caller must check `b != 0` first (division by zero
/// is an architectural fault, not a saturating/panicking ALU result).
pub fn div(a: u64, b: u64) -> AluResult {
    let value = a / b;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: false,
            v: false,
        },
    }
}

pub fn inc(a: u64) -> AluResult {
    let value = a.wrapping_add(1);
    let v = a == i64::MAX as u64;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: false, v },
    }
}

pub fn dec(a: u64) -> AluResult {
    let value = a.wrapping_sub(1);
    let v = a == i64::MIN as u64;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: false, v },
    }
}

pub fn and(a: u64, b: u64) -> AluResult {
    let value = a & b;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: false,
            v: false,
        },
    }
}

pub fn or(a: u64, b: u64) -> AluResult {
    let value = a | b;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: false,
            v: false,
        },
    }
}

pub fn xor(a: u64, b: u64) -> AluResult {
    let value = a ^ b;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: false,
            v: false,
        },
    }
}

pub fn not(a: u64) -> AluResult {
    let value = !a;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: false,
            v: false,
        },
    }
}

const SHIFT_AMOUNT_MASK: u64 = 0x3F;

pub fn shl(a: u64, count: u64) -> AluResult {
    let amount = (count & SHIFT_AMOUNT_MASK) as u32;
    let value = a.wrapping_shl(amount);
    let carry = amount > 0 && amount <= 64 && ((a >> (64 - amount.max(1))) & 1) != 0;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: carry,
            v: false,
        },
    }
}

pub fn shr(a: u64, count: u64) -> AluResult {
    let amount = (count & SHIFT_AMOUNT_MASK) as u32;
    let value = a.wrapping_shr(amount);
    let carry = amount > 0 && ((a >> (amount - 1)) & 1) != 0;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: carry,
            v: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sets_carry_on_overflow() {
        let r = add(u64::MAX, 1);
        assert_eq!(r.value, 0);
        assert!(r.flags.c);
        assert!(r.flags.z);
    }

    #[test]
    fn sub_sets_borrow_when_a_less_than_b() {
        let r = sub(1, 2);
        assert_eq!(r.value, u64::MAX);
        assert!(r.flags.c);
    }

    #[test]
    fn flags_bits_roundtrip() {
        let f = Flags {
            z: true,
            n: false,
            c: true,
            v: false,
        };
        assert_eq!(Flags::from_bits(f.to_bits()), f);
    }

    #[test]
    fn shl_shr_basic() {
        assert_eq!(shl(1, 4).value, 16);
        assert_eq!(shr(16, 4).value, 1);
    }
}
