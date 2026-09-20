// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Bitfield extraction/insertion and sign-extension helpers.
//!
//! The pre-expansion `jxcl` crate never had a dedicated bit-manipulation
//! module, but it repeatedly needed this exact family of operations
//! inline: `jxcl/src/isa/constants.rs`'s `SHIFT_AMOUNT_MASK` masks a
//! shift count down to its low 6 bits (`value & mask`, i.e. extracting a
//! bitfield), and `jxcl/src/encoding/decoder.rs` widens fixed-width
//! signed fields (`i32::from_le_bytes`) which is sign extension by
//! another name. This crate factors that repeated pattern into real,
//! tested, general primitives so `jxcl-encoding`/`jxcl-decoding` (and,
//! outside this batch, `jxcl-alu`'s shift/rotate and `jxcl-relocations`'
//! patch-in-place logic) share one implementation instead of each
//! reinventing masking/shifting arithmetic.
//!
//! Bit position `0` is always the least-significant bit.
#![forbid(unsafe_code)]

use jxcl_types::Word;

/// Extract a `width`-bit field starting at bit `offset` from `value`,
/// returned right-justified (i.e. as if the field were bits `0..width`).
///
/// `offset + width` may not exceed 64. `width == 0` always yields `0`.
///
/// # Panics
/// Panics if `offset + width > 64` -- an out-of-range bitfield is a
/// programming error in the caller (a fixed, known-at-compile-time
/// layout), not a runtime/data condition to recover from.
pub fn extract_bits(value: u64, offset: u32, width: u32) -> u64 {
    assert!(
        offset.checked_add(width).is_some_and(|end| end <= 64),
        "extract_bits: offset {} + width {} exceeds 64 bits",
        offset,
        width
    );
    if width == 0 {
        return 0;
    }
    let mask: u64 = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    (value >> offset) & mask
}

/// Insert `field`'s low `width` bits into `value` at bit `offset`,
/// leaving every other bit of `value` unchanged, and return the result.
///
/// Bits of `field` above `width` are silently discarded (masked off)
/// rather than rejected -- callers that need to detect a too-large field
/// should check `field <= max_value_for(width)` themselves, or use
/// `extract_bits` on the result and compare.
///
/// # Panics
/// Panics if `offset + width > 64`, for the same reason as
/// [`extract_bits`].
pub fn insert_bits(value: u64, offset: u32, width: u32, field: u64) -> u64 {
    assert!(
        offset.checked_add(width).is_some_and(|end| end <= 64),
        "insert_bits: offset {} + width {} exceeds 64 bits",
        offset,
        width
    );
    if width == 0 {
        return value;
    }
    let mask: u64 = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let cleared = value & !(mask << offset);
    cleared | ((field & mask) << offset)
}

/// The largest value representable in `width` bits (`0` for `width == 0`).
pub fn max_value_for(width: u32) -> u64 {
    if width == 0 {
        0
    } else if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// Sign-extend the low `width` bits of `value` to a full 64-bit signed
/// value, matching the same convention `i32::from_le_bytes`/
/// `i16::from_le_bytes` use for JXCL's fixed-width signed fields
/// (branch displacements, memory-operand displacements) -- this is the
/// general form of that same operation, usable for widths those native
/// integer types don't cover (e.g. a hypothetical 12-bit immediate).
///
/// # Panics
/// Panics if `width == 0` or `width > 64` (there is no sign bit to
/// extend from).
pub fn sign_extend(value: u64, width: u32) -> i64 {
    assert!(
        width > 0 && width <= 64,
        "sign_extend: width {} must be in 1..=64",
        width
    );
    if width == 64 {
        return value as i64;
    }
    let field = extract_bits(value, 0, width);
    let sign_bit = 1u64 << (width - 1);
    if field & sign_bit != 0 {
        // Set every bit above the field to 1.
        (field | !max_value_for(width)) as i64
    } else {
        field as i64
    }
}

/// [`extract_bits`]/[`insert_bits`] over a [`jxcl_types::Word`], for
/// callers that already carry values in that newtype rather than a raw
/// `u64`.
pub fn extract_bits_word(value: Word, offset: u32, width: u32) -> u64 {
    extract_bits(value.get(), offset, width)
}

pub fn insert_bits_word(value: Word, offset: u32, width: u32, field: u64) -> Word {
    Word::new(insert_bits(value.get(), offset, width, field))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bits_basic() {
        // 0b1011_0100
        let v = 0b1011_0100u64;
        assert_eq!(extract_bits(v, 0, 4), 0b0100);
        assert_eq!(extract_bits(v, 4, 4), 0b1011);
        assert_eq!(extract_bits(v, 2, 3), 0b101);
    }

    #[test]
    fn extract_bits_zero_width_is_zero() {
        assert_eq!(extract_bits(u64::MAX, 5, 0), 0);
    }

    #[test]
    fn extract_bits_full_width() {
        assert_eq!(extract_bits(u64::MAX, 0, 64), u64::MAX);
    }

    #[test]
    #[should_panic(expected = "exceeds 64 bits")]
    fn extract_bits_out_of_range_panics() {
        extract_bits(0, 60, 10);
    }

    #[test]
    fn insert_bits_basic() {
        let v = 0u64;
        let v = insert_bits(v, 0, 4, 0xF);
        assert_eq!(v, 0x0F);
        let v = insert_bits(v, 4, 4, 0xA);
        assert_eq!(v, 0xAF);
    }

    #[test]
    fn insert_bits_masks_oversized_field() {
        // Only the low 4 bits of 0xFF (0xF) should land.
        let v = insert_bits(0, 0, 4, 0xFF);
        assert_eq!(v, 0xF);
    }

    #[test]
    fn insert_bits_leaves_other_bits_untouched() {
        let v = 0b1111_1111u64;
        let v = insert_bits(v, 4, 4, 0b0000);
        assert_eq!(v, 0b0000_1111);
    }

    #[test]
    fn max_value_for_widths() {
        assert_eq!(max_value_for(0), 0);
        assert_eq!(max_value_for(1), 1);
        assert_eq!(max_value_for(8), 0xFF);
        assert_eq!(max_value_for(64), u64::MAX);
    }

    #[test]
    fn sign_extend_matches_native_i32_from_le_bytes() {
        // The exact width jxcl-decoding widens BranchImm32/RMem/MemR/Cas
        // displacements from -- this is the property that justifies
        // factoring the pattern out.
        for raw in [0i32, -1, 1, i32::MIN, i32::MAX, -12345, 12345] {
            let bytes = raw.to_le_bytes();
            let native = i32::from_le_bytes(bytes) as i64;
            let via_bitops = sign_extend(raw as u32 as u64, 32);
            assert_eq!(via_bitops, native, "mismatch for raw={raw}");
        }
    }

    #[test]
    fn sign_extend_16_bit() {
        assert_eq!(sign_extend(0x0001, 16), 1);
        assert_eq!(sign_extend(0xFFFF, 16), -1);
        assert_eq!(sign_extend(0x8000, 16), -32768);
        assert_eq!(sign_extend(0x7FFF, 16), 32767);
    }

    #[test]
    fn sign_extend_full_width_is_reinterpretation() {
        assert_eq!(sign_extend(u64::MAX, 64), -1);
    }

    #[test]
    #[should_panic(expected = "must be in 1..=64")]
    fn sign_extend_zero_width_panics() {
        sign_extend(0, 0);
    }

    /// Property: inserting a field and then extracting it back at the
    /// same offset/width always returns the (masked) field, for every
    /// offset/width/value combination that fits in a byte -- exhaustive
    /// rather than sampled, since the domain is small enough to cover
    /// completely and that's strictly stronger evidence.
    #[test]
    fn property_insert_then_extract_roundtrips() {
        for width in 1..=8u32 {
            let max_field = max_value_for(width);
            for offset in 0..=(16 - width) {
                for field in 0..=max_field {
                    let inserted = insert_bits(0, offset, width, field);
                    let extracted = extract_bits(inserted, offset, width);
                    assert_eq!(
                        extracted, field,
                        "roundtrip failed at offset={offset} width={width} field={field}"
                    );
                }
            }
        }
    }

    /// Property: inserting a field never disturbs bits outside its
    /// [offset, offset+width) window.
    #[test]
    fn property_insert_bits_is_local() {
        let base = 0b1010_1010_1010_1010u64;
        for width in 1..=6u32 {
            for offset in 0..=(16 - width) {
                let inserted = insert_bits(base, offset, width, 0);
                for bit in 0..16u32 {
                    if bit < offset || bit >= offset + width {
                        assert_eq!(
                            extract_bits(inserted, bit, 1),
                            extract_bits(base, bit, 1),
                            "bit {bit} disturbed by insert at offset={offset} width={width}"
                        );
                    }
                }
            }
        }
    }

    /// Property: sign_extend followed by taking the low `width` bits
    /// back always recovers the original field, for every field/width
    /// combination up to a byte.
    #[test]
    fn property_sign_extend_then_truncate_roundtrips() {
        for width in 1..=16u32 {
            let max_field = max_value_for(width).min(4095); // keep the loop fast
            for field in 0..=max_field {
                let extended = sign_extend(field, width);
                let truncated = extract_bits(extended as u64, 0, width);
                assert_eq!(
                    truncated, field,
                    "sign_extend/truncate roundtrip failed at width={width} field={field}"
                );
            }
        }
    }

    #[test]
    fn word_wrapper_helpers_match_raw_functions() {
        let w = Word::new(0xABCD);
        assert_eq!(extract_bits_word(w, 0, 8), 0xCD);
        let updated = insert_bits_word(w, 0, 8, 0xFF);
        assert_eq!(updated, Word::new(0xABFF));
    }
}
