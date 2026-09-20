// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Arithmetic/logic unit semantics: add/sub/mul/div/shift/bitwise, with flag updates.
//!
//! Owns: All arithmetic semantics -- no other crate performs ALU-equivalent computation independently.
//!
//! Extracted from `jxcl/src/alu.rs`. Pure, host-independent 64-bit integer operations
//! with explicit carry/overflow/shift semantics, testable in isolation from the execution engine.
//!
//! Width/signedness/overflow/truncation/carry/borrow/shift/comparison behavior (spec §3, §23)
//! is fixed here:
//! - All operands and results are 64-bit words; signed interpretation is via `as i64` where needed.
//! - Arithmetic wraps modulo 2^64 (wrapping_*); the wrapped result is always written back.
//! - Carry follows the x86 convention: for ADD/ADC it is the unsigned carry-out of bit 63;
//!   for SUB/SBC/CMP/NEG it is the unsigned borrow.
//! - Overflow (V) is the signed overflow of the operation.
//! - Shift/rotate counts are masked to the low 6 bits of the count operand (never a fault).
#![forbid(unsafe_code)]

use jxcl_types::Word;
use jxcl_flags::Flags;
use jxcl_constants::SHIFT_AMOUNT_MASK;

/// Bundles a 64-bit result with the four condition flags an ALU operation may produce.
/// Callers apply only the flags declared by the instruction's FlagEffect mask (spec §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult {
    pub value: Word,
    pub flags: Flags,
}

fn zn(value: Word) -> (bool, bool) {
    Flags::zn_of(value.get())
}

/// Add without carry.
pub fn add(a: Word, b: Word) -> AluResult {
    let (value, carry) = a.overflowing_add(b);
    let (_, v) = a.signed_overflowing_add(b);
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: carry, v },
    }
}

/// Add with carry.
pub fn adc(a: Word, b: Word, carry_in: bool) -> AluResult {
    let (sum1, c1) = a.overflowing_add(b);
    let (sum2, c2) = sum1.overflowing_add(Word::new(carry_in as u64));
    let (_, v1) = a.signed_overflowing_add(b);
    let (_, v2) = sum1.signed_overflowing_add(Word::new(carry_in as u64));
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

/// Subtract.
pub fn sub(a: Word, b: Word) -> AluResult {
    let value = a.wrapping_sub(b);
    let borrow = a.get() < b.get();
    let (_, v) = (a.as_i64() as i128).overflowing_sub(b.as_i64() as i128);
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: borrow, v },
    }
}

/// Subtract with borrow.
pub fn sbc(a: Word, b: Word, borrow_in: bool) -> AluResult {
    let full = (a.get() as u128)
        .wrapping_sub(b.get() as u128)
        .wrapping_sub(borrow_in as u128);
    let value = Word::new(full as u64);
    let rhs = (b.get() as u128) + (borrow_in as u128);
    let borrow = (a.get() as u128) < rhs;
    let (t1, v1) = (a.as_i64() as i128).overflowing_sub(b.as_i64() as i128);
    let (_, v2) = t1.overflowing_sub(borrow_in as i128);
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

/// Negate (two's complement).
pub fn neg(a: Word) -> AluResult {
    sub(Word::ZERO, a)
}

/// Increment by 1.
pub fn inc(a: Word) -> AluResult {
    let value = a.wrapping_add(Word::new(1));
    let v = a.get() == i64::MAX as u64;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: false, v },
    }
}

/// Decrement by 1.
pub fn dec(a: Word) -> AluResult {
    let value = a.wrapping_sub(Word::new(1));
    let v = a.get() == i64::MIN as u64;
    let (z, n) = zn(value);
    AluResult {
        value,
        flags: Flags { z, n, c: false, v },
    }
}

/// Unsigned 64x64->128 multiply, low 64 bits kept, c/v set together when overflow.
pub fn mul(a: Word, b: Word) -> AluResult {
    let full = (a.get() as u128) * (b.get() as u128);
    let value = Word::new(full as u64);
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
pub fn mulh(a: Word, b: Word) -> AluResult {
    let full = (a.get() as u128) * (b.get() as u128);
    let value = Word::new((full >> 64) as u64);
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

/// Unsigned division. Caller must check b != 0.
pub fn div(a: Word, b: Word) -> AluResult {
    let value = Word::new(a.get() / b.get());
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

/// Unsigned remainder.
pub fn rem(a: Word, b: Word) -> AluResult {
    let value = Word::new(a.get() % b.get());
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

/// Bitwise AND.
pub fn and(a: Word, b: Word) -> AluResult {
    let value = Word::new(a.get() & b.get());
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

/// Bitwise OR.
pub fn or(a: Word, b: Word) -> AluResult {
    let value = Word::new(a.get() | b.get());
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

/// Bitwise XOR.
pub fn xor(a: Word, b: Word) -> AluResult {
    let value = Word::new(a.get() ^ b.get());
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

/// Triple XOR.
pub fn xor3(a: Word, b: Word, c: Word) -> AluResult {
    let value = Word::new(a.get() ^ b.get() ^ c.get());
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

/// Bitwise NOT.
pub fn not(a: Word) -> AluResult {
    let value = Word::new(!a.get());
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

/// Bitwise NAND.
pub fn nand(a: Word, b: Word) -> AluResult {
    let and_result = and(a, b);
    not(and_result.value)
}

/// Bitwise NOR.
pub fn nor(a: Word, b: Word) -> AluResult {
    let or_result = or(a, b);
    not(or_result.value)
}

/// Compare: like SUB, but caller discards value and keeps only flags.
pub fn cmp(a: Word, b: Word) -> AluResult {
    sub(a, b)
}

/// Test: like AND, but caller discards value and keeps only flags.
pub fn test(a: Word, b: Word) -> AluResult {
    and(a, b)
}

fn shift_amount(count: Word) -> u32 {
    (count.get() & SHIFT_AMOUNT_MASK) as u32
}

/// Logical shift left.
pub fn shl(a: Word, count: Word) -> AluResult {
    let amt = shift_amount(count);
    let value = if amt == 0 { a } else { Word::new(a.get().wrapping_shl(amt)) };
    let carry = if amt == 0 {
        false
    } else {
        (a.get() >> (64 - amt)) & 1 != 0
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

/// Logical shift right.
pub fn shr(a: Word, count: Word) -> AluResult {
    let amt = shift_amount(count);
    let value = if amt == 0 { a } else { Word::new(a.get().wrapping_shr(amt)) };
    let carry = if amt == 0 {
        false
    } else {
        (a.get() >> (amt - 1)) & 1 != 0
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

/// Arithmetic shift right.
pub fn sar(a: Word, count: Word) -> AluResult {
    let amt = shift_amount(count);
    let signed = a.as_i64();
    let value = if amt == 0 {
        signed
    } else {
        signed.wrapping_shr(amt)
    } as u64;
    let value = Word::new(value);
    let carry = if amt == 0 {
        false
    } else {
        (a.get() >> (amt - 1)) & 1 != 0
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

/// Rotate left.
pub fn rol(a: Word, count: Word) -> AluResult {
    let amt = shift_amount(count);
    let value = Word::new(a.get().rotate_left(amt));
    let carry = value.get() & 1 != 0;
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

/// Rotate right.
pub fn ror(a: Word, count: Word) -> AluResult {
    let amt = shift_amount(count);
    let value = Word::new(a.get().rotate_right(amt));
    let carry = (value.get() >> 63) & 1 != 0;
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
        let r = add(Word::MAX, Word::new(1));
        assert_eq!(r.value, Word::ZERO);
        assert!(r.flags.z);
        assert!(r.flags.c);
        assert!(!r.flags.v);

        let r = add(Word::new(i64::MAX as u64), Word::new(1));
        assert!(r.flags.v, "signed overflow of MAX+1 must set V");
    }

    #[test]
    fn sub_borrow() {
        let r = sub(Word::ZERO, Word::new(1));
        assert_eq!(r.value, Word::MAX);
        assert!(r.flags.c, "borrow expected");
        assert!(r.flags.n);
    }

    #[test]
    fn mul_overflow_detected() {
        let r = mul(Word::MAX, Word::new(2));
        assert!(r.flags.c);
        assert!(r.flags.v);
        let r2 = mul(Word::new(2), Word::new(3));
        assert!(!r2.flags.c);
        assert_eq!(r2.value, Word::new(6));
    }

    #[test]
    fn shift_by_zero_is_identity_and_no_carry() {
        let r = shl(Word::new(0x42), Word::ZERO);
        assert_eq!(r.value, Word::new(0x42));
        assert!(!r.flags.c);
    }

    #[test]
    fn shift_amount_masks_to_six_bits() {
        let r = shl(Word::new(1), Word::new(64));
        assert_eq!(r.value, Word::new(1));
    }

    #[test]
    fn rotate_is_bit_preserving() {
        let r = rol(Word::new(1), Word::new(1));
        assert_eq!(r.value, Word::new(2));
        let r2 = ror(Word::new(2), Word::new(1));
        assert_eq!(r2.value, Word::new(1));
    }

    #[test]
    fn bitwise_operations() {
        let a = Word::new(0xFF00FF00);
        let b = Word::new(0x00FF00FF);

        let and_r = and(a, b);
        assert_eq!(and_r.value, Word::ZERO);
        assert!(and_r.flags.z);

        let or_r = or(a, b);
        assert_eq!(or_r.value, Word::new(0xFFFFFFFF));
        assert!(!or_r.flags.z);

        let xor_r = xor(a, b);
        assert_eq!(xor_r.value, Word::new(0xFFFFFFFF));

        let not_r = not(Word::ZERO);
        assert_eq!(not_r.value, Word::MAX);
    }

    #[test]
    fn inc_dec_wrap() {
        let r = inc(Word::MAX);
        assert_eq!(r.value, Word::ZERO);
        assert!(r.flags.z);

        let r = dec(Word::ZERO);
        assert_eq!(r.value, Word::MAX);
        assert!(r.flags.n);
    }

    #[test]
    fn division_and_remainder() {
        let r = div(Word::new(20), Word::new(3));
        assert_eq!(r.value, Word::new(6));

        let r = rem(Word::new(20), Word::new(3));
        assert_eq!(r.value, Word::new(2));
    }
}
