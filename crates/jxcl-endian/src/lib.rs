// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The ISA's fixed-endianness integer <-> byte conversions.
//!
//! Extracted from `jxcl/src/encoding/encoder.rs` and
//! `jxcl/src/encoding/decoder.rs`, which call `to_le_bytes`/
//! `from_le_bytes` directly at every multi-byte field (`imm.to_le_bytes()`
//! for `RImm64`, `disp.to_le_bytes()` for `RMem`/`MemR`/`Cas`/
//! `BranchImm32`, `u16::from_le_bytes`/`i32::from_le_bytes`/
//! `u64::from_le_bytes` in the decoder). That is little-endian, matching
//! `jxcl-constants::LITTLE_ENDIAN` (spec §10: "Default: little endian
//! unless the specification explicitly selects another format. No other
//! format is selected here.") -- this crate isolates that one
//! architectural choice into a single named place so the encoder/decoder
//! never call `to_le_bytes`/`from_le_bytes` directly, and a hypothetical
//! future big-endian ISA revision would only need to change this crate.
#![forbid(unsafe_code)]

/// Converts a fixed-width integer to and from the architecture's byte
/// order. Implemented for every integer width the ISA actually encodes
/// (`u8`/`u16`/`u32`/`u64` and their signed counterparts, since
/// displacement fields are signed).
pub trait ArchBytes: Sized + Copy {
    /// The fixed-size byte array this type converts to/from.
    type Bytes: AsRef<[u8]> + AsMut<[u8]> + Default;

    /// Convert to the architecture's byte order.
    fn to_arch_bytes(self) -> Self::Bytes;

    /// Convert from the architecture's byte order. Returns `None` if
    /// `bytes` is shorter than this type's width (it may be longer;
    /// only the first `size_of::<Self>()` bytes are consumed).
    fn from_arch_bytes(bytes: &[u8]) -> Option<Self>;
}

macro_rules! impl_arch_bytes {
    ($($t:ty => $n:expr),* $(,)?) => {
        $(
            impl ArchBytes for $t {
                type Bytes = [u8; $n];

                fn to_arch_bytes(self) -> Self::Bytes {
                    // Little-endian: matches jxcl-constants::LITTLE_ENDIAN
                    // and every `to_le_bytes()`/`from_le_bytes()` call in
                    // the pre-expansion encoder/decoder.
                    self.to_le_bytes()
                }

                fn from_arch_bytes(bytes: &[u8]) -> Option<Self> {
                    let arr: [u8; $n] = bytes.get(0..$n)?.try_into().ok()?;
                    Some(<$t>::from_le_bytes(arr))
                }
            }
        )*
    };
}

impl_arch_bytes! {
    u8 => 1, u16 => 2, u32 => 4, u64 => 8,
    i8 => 1, i16 => 2, i32 => 4, i64 => 8,
}

/// Free-function form of [`ArchBytes::to_arch_bytes`], for call sites
/// that prefer `to_arch_bytes(value)` over `value.to_arch_bytes()`
/// (matching the registry's `public_api` naming).
pub fn to_arch_bytes<T: ArchBytes>(value: T) -> T::Bytes {
    value.to_arch_bytes()
}

/// Free-function form of [`ArchBytes::from_arch_bytes`].
pub fn from_arch_bytes<T: ArchBytes>(bytes: &[u8]) -> Option<T> {
    T::from_arch_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_matches_to_le_bytes_exactly() {
        // This is the load-bearing assertion: jxcl-encoding must produce
        // byte-for-byte identical output to the pre-expansion encoder.
        let v: u64 = 0x0102_0304_0506_0708;
        assert_eq!(to_arch_bytes(v), v.to_le_bytes());
    }

    #[test]
    fn i32_matches_to_le_bytes_exactly() {
        let v: i32 = -12345;
        assert_eq!(to_arch_bytes(v), v.to_le_bytes());
    }

    #[test]
    fn u16_roundtrips() {
        let v: u16 = 0xBEEF;
        let bytes = to_arch_bytes(v);
        assert_eq!(from_arch_bytes::<u16>(&bytes), Some(v));
    }

    #[test]
    fn from_arch_bytes_rejects_short_input() {
        assert_eq!(from_arch_bytes::<u64>(&[1, 2, 3]), None);
        assert_eq!(from_arch_bytes::<u32>(&[]), None);
    }

    #[test]
    fn from_arch_bytes_ignores_trailing_bytes() {
        // decode_all reads instructions out of a shared buffer; a
        // multi-byte field only ever consumes its own width even when
        // more bytes follow.
        let bytes = [0x01u8, 0x00, 0xFF, 0xFF];
        assert_eq!(from_arch_bytes::<u16>(&bytes), Some(1u16));
    }

    #[test]
    fn property_roundtrip_every_width() {
        for v in [0u64, 1, 42, u32::MAX as u64, u64::MAX] {
            assert_eq!(from_arch_bytes::<u64>(&to_arch_bytes(v)), Some(v));
        }
        for v in [0i32, 1, -1, i32::MIN, i32::MAX] {
            assert_eq!(from_arch_bytes::<i32>(&to_arch_bytes(v)), Some(v));
        }
        for v in [0u16, 1, u16::MAX] {
            assert_eq!(from_arch_bytes::<u16>(&to_arch_bytes(v)), Some(v));
        }
        for v in [0u8, 1, u8::MAX] {
            assert_eq!(from_arch_bytes::<u8>(&to_arch_bytes(v)), Some(v));
        }
    }
}
