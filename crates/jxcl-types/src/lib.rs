// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Shared newtype wrappers for architectural primitive values (word,
//! address, register index, immediate) so every ISA/execution crate
//! agrees on one representation.
//!
//! These types intentionally carry no architectural policy (no width
//! validation against `jxcl-constants`, no register-count checking): they
//! are the raw, dependency-free vocabulary that every other crate in the
//! workspace builds on. Range/validity checks that depend on the actual
//! ISA parameters (e.g. "is this register index in range") belong to the
//! crates that own those parameters (`jxcl-registers`, `jxcl-constants`).
//!
//! Owns: the `Word`/`Address`/`RegisterIndex`/`Immediate` newtypes and
//! their arithmetic/conversion impls.
#![forbid(unsafe_code)]

use std::fmt;
use std::ops::{Add, BitAnd, BitOr, BitXor, Not, Shl, Shr, Sub};

/// A raw 64-bit architectural word: the natural size of every
/// general-purpose register and ALU operation in TLM JXCL.
///
/// Arithmetic wraps (matching two's-complement register semantics) rather
/// than panicking on overflow, since register overflow is an
/// architecturally meaningful, expected event (the ALU inspects the carry
/// out of it), not a host-language bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Word(pub u64);

impl Word {
    pub const ZERO: Word = Word(0);
    pub const MAX: Word = Word(u64::MAX);

    pub const fn new(value: u64) -> Self {
        Word(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    /// Interpret the bit pattern as a signed 64-bit value.
    pub const fn as_i64(self) -> i64 {
        self.0 as i64
    }

    pub const fn wrapping_add(self, rhs: Word) -> Word {
        Word(self.0.wrapping_add(rhs.0))
    }

    pub const fn wrapping_sub(self, rhs: Word) -> Word {
        Word(self.0.wrapping_sub(rhs.0))
    }

    /// Unsigned-overflowing add, exposing the carry flag the ALU needs.
    pub const fn overflowing_add(self, rhs: Word) -> (Word, bool) {
        let (v, carry) = self.0.overflowing_add(rhs.0);
        (Word(v), carry)
    }

    /// Signed-overflowing add, exposing the two's-complement overflow flag.
    pub const fn signed_overflowing_add(self, rhs: Word) -> (Word, bool) {
        let (v, overflow) = (self.0 as i64).overflowing_add(rhs.0 as i64);
        (Word(v as u64), overflow)
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl From<u64> for Word {
    fn from(v: u64) -> Self {
        Word(v)
    }
}
impl From<Word> for u64 {
    fn from(w: Word) -> Self {
        w.0
    }
}

impl Add for Word {
    type Output = Word;
    fn add(self, rhs: Word) -> Word {
        self.wrapping_add(rhs)
    }
}
impl Sub for Word {
    type Output = Word;
    fn sub(self, rhs: Word) -> Word {
        self.wrapping_sub(rhs)
    }
}
impl BitAnd for Word {
    type Output = Word;
    fn bitand(self, rhs: Word) -> Word {
        Word(self.0 & rhs.0)
    }
}
impl BitOr for Word {
    type Output = Word;
    fn bitor(self, rhs: Word) -> Word {
        Word(self.0 | rhs.0)
    }
}
impl BitXor for Word {
    type Output = Word;
    fn bitxor(self, rhs: Word) -> Word {
        Word(self.0 ^ rhs.0)
    }
}
impl Not for Word {
    type Output = Word;
    fn not(self) -> Word {
        Word(!self.0)
    }
}
impl Shl<u32> for Word {
    type Output = Word;
    fn shl(self, rhs: u32) -> Word {
        Word(self.0.wrapping_shl(rhs))
    }
}
impl Shr<u32> for Word {
    type Output = Word;
    fn shr(self, rhs: u32) -> Word {
        Word(self.0.wrapping_shr(rhs))
    }
}

/// A 64-bit virtual/physical address in the architectural address space.
///
/// Kept as a distinct type from [`Word`] even though both wrap a `u64`:
/// a `Word` is a value living in a register or memory cell, while an
/// `Address` names a location. Mixing the two (e.g. treating a loaded
/// value as though it were automatically an address) is exactly the kind
/// of confusion a newtype boundary is meant to catch at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Address(pub u64);

impl Address {
    pub const ZERO: Address = Address(0);

    pub const fn new(value: u64) -> Self {
        Address(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    /// Offset this address by a signed displacement, wrapping on overflow
    /// (matching the reference machine's flat, wraparound address space).
    pub const fn offset(self, displacement: i64) -> Address {
        Address(self.0.wrapping_add(displacement as u64))
    }

    /// Number of bytes from `self` to `other` (may be negative).
    pub const fn distance_to(self, other: Address) -> i64 {
        other.0.wrapping_sub(self.0) as i64
    }

    pub const fn is_aligned_to(self, alignment: u64) -> bool {
        alignment != 0 && self.0.is_multiple_of(alignment)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl From<u64> for Address {
    fn from(v: u64) -> Self {
        Address(v)
    }
}
impl From<Address> for u64 {
    fn from(a: Address) -> Self {
        a.0
    }
}
impl From<Word> for Address {
    fn from(w: Word) -> Self {
        Address(w.0)
    }
}
impl From<Address> for Word {
    fn from(a: Address) -> Self {
        Word(a.0)
    }
}

impl Add<i64> for Address {
    type Output = Address;
    fn add(self, rhs: i64) -> Address {
        self.offset(rhs)
    }
}

/// A general-purpose register operand index (`0..NUM_GP_REGISTERS`, per
/// `jxcl-constants`). This crate does not know how many registers exist
/// -- it just carries the raw index -- so range validation lives in
/// `jxcl-registers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegisterIndex(pub u8);

impl RegisterIndex {
    pub const fn new(index: u8) -> Self {
        RegisterIndex(index)
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for RegisterIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R{}", self.0)
    }
}

impl From<u8> for RegisterIndex {
    fn from(v: u8) -> Self {
        RegisterIndex(v)
    }
}
impl From<RegisterIndex> for u8 {
    fn from(r: RegisterIndex) -> Self {
        r.0
    }
}

/// An immediate value carried directly in an instruction's encoding
/// (as opposed to a value fetched from a register or memory).
///
/// Backed by a `u64` so it can hold the full-width `MOVI` immediate;
/// narrower encoded immediates (16/32-bit) are sign- or zero-extended
/// into it by their owning crate (`jxcl-operands`/`jxcl-encoding`) before
/// wrapping here, so `Immediate` itself only has to reason about one
/// width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Immediate(pub u64);

impl Immediate {
    pub const ZERO: Immediate = Immediate(0);

    pub const fn new(value: u64) -> Self {
        Immediate(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn as_signed(self) -> i64 {
        self.0 as i64
    }

    /// Build an `Immediate` from a signed value that should be
    /// sign-extended to the full 64-bit word (e.g. a decoded 32-bit
    /// branch displacement being widened for arithmetic).
    pub const fn from_signed(value: i64) -> Self {
        Immediate(value as u64)
    }

    pub const fn wrapping_add(self, rhs: Immediate) -> Immediate {
        Immediate(self.0.wrapping_add(rhs.0))
    }
}

impl fmt::Display for Immediate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_signed())
    }
}

impl From<u64> for Immediate {
    fn from(v: u64) -> Self {
        Immediate(v)
    }
}
impl From<i64> for Immediate {
    fn from(v: i64) -> Self {
        Immediate::from_signed(v)
    }
}
impl From<i32> for Immediate {
    fn from(v: i32) -> Self {
        Immediate::from_signed(v as i64)
    }
}
impl From<u16> for Immediate {
    fn from(v: u16) -> Self {
        Immediate(v as u64)
    }
}
impl From<Immediate> for u64 {
    fn from(i: Immediate) -> Self {
        i.0
    }
}

impl Add for Immediate {
    type Output = Immediate;
    fn add(self, rhs: Immediate) -> Immediate {
        self.wrapping_add(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_wrapping_arithmetic() {
        assert_eq!(Word::MAX.wrapping_add(Word::new(1)), Word::ZERO);
        assert_eq!(Word::ZERO.wrapping_sub(Word::new(1)), Word::MAX);
        assert_eq!(Word::new(1) + Word::new(2), Word::new(3));
    }

    #[test]
    fn word_overflow_flags() {
        let (v, carry) = Word::MAX.overflowing_add(Word::new(1));
        assert_eq!(v, Word::ZERO);
        assert!(carry);

        let (v, carry) = Word::new(1).overflowing_add(Word::new(1));
        assert_eq!(v, Word::new(2));
        assert!(!carry);

        let (_, overflow) = Word::new(i64::MAX as u64).signed_overflowing_add(Word::new(1));
        assert!(overflow);
    }

    #[test]
    fn word_bitwise_and_shift() {
        let a = Word::new(0b1100);
        let b = Word::new(0b1010);
        assert_eq!(a & b, Word::new(0b1000));
        assert_eq!(a | b, Word::new(0b1110));
        assert_eq!(a ^ b, Word::new(0b0110));
        assert_eq!(Word::new(1) << 4, Word::new(0b10000));
        assert_eq!(Word::new(0b10000) >> 4, Word::new(1));
    }

    #[test]
    fn word_display_is_hex() {
        assert_eq!(format!("{}", Word::new(255)), "0x00000000000000ff");
    }

    #[test]
    fn address_offset_and_distance() {
        let base = Address::new(1000);
        let target = base.offset(-8);
        assert_eq!(target, Address::new(992));
        assert_eq!(base.distance_to(target), -8);
        assert_eq!(target.distance_to(base), 8);
    }

    #[test]
    fn address_wraps_at_the_top_of_the_space() {
        let top = Address::new(u64::MAX);
        assert_eq!(top.offset(1), Address::ZERO);
    }

    #[test]
    fn address_alignment_check() {
        assert!(Address::new(64).is_aligned_to(8));
        assert!(!Address::new(65).is_aligned_to(8));
        assert!(!Address::new(8).is_aligned_to(0));
    }

    #[test]
    fn address_word_roundtrip() {
        let w = Word::new(42);
        let a: Address = w.into();
        let back: Word = a.into();
        assert_eq!(w, back);
    }

    #[test]
    fn register_index_conversions() {
        let r = RegisterIndex::new(7);
        assert_eq!(r.get(), 7);
        assert_eq!(r.as_usize(), 7usize);
        assert_eq!(format!("{}", r), "R7");
        let raw: u8 = r.into();
        assert_eq!(raw, 7);
    }

    #[test]
    fn immediate_signed_and_unsigned_views() {
        let imm = Immediate::from_signed(-1);
        assert_eq!(imm.get(), u64::MAX);
        assert_eq!(imm.as_signed(), -1);
        assert_eq!(format!("{}", imm), "-1");
    }

    #[test]
    fn immediate_from_narrow_types_widens_correctly() {
        let from_i32: Immediate = (-4i32).into();
        assert_eq!(from_i32.as_signed(), -4);
        let from_u16: Immediate = 65535u16.into();
        assert_eq!(from_u16.get(), 65535);
    }

    #[test]
    fn immediate_addition_wraps() {
        let a = Immediate::new(u64::MAX);
        let b = Immediate::new(1);
        assert_eq!(a + b, Immediate::ZERO);
    }
}
