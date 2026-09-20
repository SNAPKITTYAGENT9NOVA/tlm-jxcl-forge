// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The Arithmetic Logic Unit (spec §23): pure, host-independent 64-bit
//! integer operations with explicit carry/overflow/shift semantics,
//! testable in isolation from the execution engine (spec §23: "Implement
//! the ALU independently enough to test it.").
//!
//! Width/signedness/overflow/truncation/carry/borrow/shift/comparison
//! behavior (spec §3) is fixed here, once, for every caller:
//!
//! - All operands and results are 64-bit words (`u64`); signed
//!   interpretation is via `as i64` where semantics call for it.
//! - Arithmetic wraps modulo 2^64 (`wrapping_*`); the wrapped result is
//!   always what's written back, never a panic.
//! - **Carry** follows the x86 convention: for ADD/ADC it is the
//!   *unsigned* carry-out of bit 63; for SUB/SBC/CMP/NEG it is the
//!   unsigned *borrow* (`1` iff the true mathematical difference is
//!   negative).
//! - **Overflow (V)** is the *signed* overflow of the operation.
//! - Shift/rotate counts are masked to the low 6 bits of the count
//!   operand (`constants::SHIFT_AMOUNT_MASK`) — never an architectural
//!   fault, always a well-defined result (spec §3 "shift behavior").

use crate::isa::constants::SHIFT_AMOUNT_MASK;
use crate::isa::flags::Flags;

/// Bundles a 64-bit result with the four condition flags an ALU op may
/// have produced. Callers apply only the flags declared by the
/// instruction's `FlagEffect` mask (spec §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult {
    pub value: u64,
    pub flags: Flags,
}

fn zn(value: u64) -> (bool, bool) {
    Flags::zn_of(value)
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

pub fn adc(a: u64, b: u64, carry_in: bool) -> AluResult {
    let (sum1, c1) = a.overflowing_add(b);
    let (sum2, c2) = sum1.overflowing_add(carry_in as u64);
    let (svum1, v1) = (a as i64).overflowing_add(b as i64);
    let (_, v2) = svum1.overflowing_add(carry_in as i64);
    let (z, n) = zn(sum2);
    AluResult {
        value: sum2,
        flags: Flags {
            z,
            n,
            c: c1 || c2,
            v: v1 || v2,
        },
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

pub fn sbc(a: u64, b: u64, borrow_in: bool) -> AluResult {
    let full = (a as u128)
        .wrapping_sub(b as u128)
        .wrapping_sub(borrow_in as u128);
    let value = full as u64;
    // Borrow occurred iff a < b + borrow_in, computed without overflow.
    let rhs = (b as u128) + (borrow_in as u128);
    let borrow = (a as u128) < rhs;
    let (t1, v1) = (a as i64).overflowing_sub(b as i64);
    let (_, v2) = t1.overflowing_sub(borrow_in as i64);
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: borrow,
            v: v1 || v2,
        },
    }
}

pub fn neg(a: u64) -> AluResult {
    sub(0, a)
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

/// Unsigned 64x64->128 multiply, low 64 bits kept, `c`/`v` set together
/// when the discarded high bits are nonzero (documented simplification:
/// MUL does not distinguish signed vs. unsigned overflow, spec §23).
pub fn mul(a: u64, b: u64) -> AluResult {
    let full = (a as u128) * (b as u128);
    let value = full as u64;
    let overflow = (full >> 64) != 0;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags {
            z,
            n,
            c: overflow,
            v: overflow,
        },
    }
}

/// High 64 bits of the unsigned 128-bit product.
pub fn mulh(a: u64, b: u64) -> AluResult {
    let full = (a as u128) * (b as u128);
    let value = (full >> 64) as u64;
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

/// Unsigned division. Caller must check `b != 0` (spec §11
/// `DIVIDE_BY_ZERO` is an architectural fault, raised by the execution
/// layer, not encoded as a Rust panic here).
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

pub fn rem(a: u64, b: u64) -> AluResult {
    let value = a % b;
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

pub fn xor3(a: u64, b: u64, c: u64) -> AluResult {
    let value = a ^ b ^ c;
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

pub fn nand(a: u64, b: u64) -> AluResult {
    not(a & b)
}

pub fn nor(a: u64, b: u64) -> AluResult {
    not(a | b)
}

/// CMP: like SUB, but the caller (execution engine) discards `value` and
/// keeps only `flags`.
pub fn cmp(a: u64, b: u64) -> AluResult {
    sub(a, b)
}

/// TEST: like AND, but the caller discards `value` and keeps only `flags`.
pub fn test(a: u64, b: u64) -> AluResult {
    and(a, b)
}

fn shift_amount(count: u64) -> u32 {
    (count & SHIFT_AMOUNT_MASK) as u32
}

pub fn shl(a: u64, count: u64) -> AluResult {
    let amt = shift_amount(count);
    let value = if amt == 0 { a } else { a.wrapping_shl(amt) };
    let carry = if amt == 0 {
        false
    } else {
        (a >> (64 - amt)) & 1 != 0
    };
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
    let amt = shift_amount(count);
    let value = if amt == 0 { a } else { a.wrapping_shr(amt) };
    let carry = if amt == 0 {
        false
    } else {
        (a >> (amt - 1)) & 1 != 0
    };
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

pub fn sar(a: u64, count: u64) -> AluResult {
    let amt = shift_amount(count);
    let signed = a as i64;
    let value = if amt == 0 {
        signed
    } else {
        signed.wrapping_shr(amt)
    } as u64;
    let carry = if amt == 0 {
        false
    } else {
        (a >> (amt - 1)) & 1 != 0
    };
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

pub fn rol(a: u64, count: u64) -> AluResult {
    let amt = shift_amount(count);
    let value = a.rotate_left(amt);
    let carry = value & 1 != 0;
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

pub fn ror(a: u64, count: u64) -> AluResult {
    let amt = shift_amount(count);
    let value = a.rotate_right(amt);
    let carry = (value >> 63) & 1 != 0;
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
    fn add_carry_and_overflow() {
        let r = add(u64::MAX, 1);
        assert_eq!(r.value, 0);
        assert!(r.flags.z);
        assert!(r.flags.c);
        assert!(!r.flags.v);

        let r = add(i64::MAX as u64, 1);
        assert!(r.flags.v, "signed overflow of MAX+1 must set V");
    }

    #[test]
    fn sub_borrow() {
        let r = sub(0, 1);
        assert_eq!(r.value, u64::MAX);
        assert!(r.flags.c, "borrow expected");
        assert!(r.flags.n);
    }

    #[test]
    fn mul_overflow_detected() {
        let r = mul(u64::MAX, 2);
        assert!(r.flags.c);
        assert!(r.flags.v);
        let r2 = mul(2, 3);
        assert!(!r2.flags.c);
        assert_eq!(r2.value, 6);
    }

    #[test]
    fn shift_by_zero_is_identity_and_no_carry() {
        let r = shl(0x42, 0);
        assert_eq!(r.value, 0x42);
        assert!(!r.flags.c);
    }

    #[test]
    fn shift_amount_masks_to_six_bits() {
        // 64 mod 64 == 0 -> identity shift.
        let r = shl(1, 64);
        assert_eq!(r.value, 1);
    }

    #[test]
    fn rotate_is_bit_preserving() {
        let r = rol(1, 1);
        assert_eq!(r.value, 2);
        let r2 = ror(2, 1);
        assert_eq!(r2.value, 1);
    }
}
