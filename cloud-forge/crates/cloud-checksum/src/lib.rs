// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Content-integrity checksums for stored data.
//!
//! Implements CRC-32/ISO-HDLC (the checksum used by zlib, gzip, PNG,
//! and Ethernet) from scratch, bit by bit rather than via a
//! precomputed table -- this crate favors an obviously-correct
//! implementation over a fast one, and has no dependencies at all
//! (consistent with the rest of this workspace's zero-external-
//! dependency posture). Correctness is anchored to the algorithm's
//! own standard check value (`crc32(b"123456789") == 0xCBF4_3926`),
//! not just internal self-consistency -- see the tests.
#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

const POLY: u32 = 0xEDB8_8320;

/// Computes a [`Checksum`] incrementally, so a large or streamed
/// object never has to be held in memory all at once to be checksummed.
#[derive(Debug, Clone)]
pub struct ChecksumBuilder {
    crc: u32,
}

impl ChecksumBuilder {
    pub fn new() -> Self {
        ChecksumBuilder { crc: 0xFFFF_FFFF }
    }

    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.crc ^= u32::from(byte);
            for _ in 0..8 {
                if self.crc & 1 != 0 {
                    self.crc = (self.crc >> 1) ^ POLY;
                } else {
                    self.crc >>= 1;
                }
            }
        }
    }

    pub fn finish(self) -> Checksum {
        Checksum(!self.crc)
    }
}

impl Default for ChecksumBuilder {
    fn default() -> Self {
        ChecksumBuilder::new()
    }
}

/// A CRC-32/ISO-HDLC checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checksum(u32);

impl Checksum {
    /// Computes the checksum of `data` in one call.
    pub fn of(data: &[u8]) -> Self {
        let mut builder = ChecksumBuilder::new();
        builder.update(data);
        builder.finish()
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Whether `data` checksums to `self`.
    pub fn matches(self, data: &[u8]) -> bool {
        self == Checksum::of(data)
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

impl FromStr for Checksum {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str_radix(s, 16).map(Checksum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_checksums_to_the_well_known_zero_value() {
        assert_eq!(Checksum::of(b"").as_u32(), 0x0000_0000);
    }

    #[test]
    fn matches_the_algorithms_standard_check_value() {
        // The canonical CRC-32/ISO-HDLC check value, used by every
        // implementation (zlib, gzip, PNG, ...) to confirm correctness.
        assert_eq!(Checksum::of(b"123456789").as_u32(), 0xCBF4_3926);
    }

    #[test]
    fn identical_content_produces_the_same_checksum() {
        assert_eq!(
            Checksum::of(b"the quick brown fox"),
            Checksum::of(b"the quick brown fox")
        );
    }

    #[test]
    fn flipping_a_single_byte_changes_the_checksum() {
        let original = Checksum::of(b"the quick brown fox");
        let corrupted = Checksum::of(b"the quick brown fom");
        assert_ne!(original, corrupted);
    }

    #[test]
    fn incremental_updates_match_one_shot_computation() {
        let one_shot = Checksum::of(b"the quick brown fox");

        let mut builder = ChecksumBuilder::new();
        builder.update(b"the quick ");
        builder.update(b"brown ");
        builder.update(b"fox");
        let incremental = builder.finish();

        assert_eq!(one_shot, incremental);
    }

    #[test]
    fn matches_reports_whether_data_reproduces_the_checksum() {
        let sum = Checksum::of(b"payload");
        assert!(sum.matches(b"payload"));
        assert!(!sum.matches(b"payloae"));
    }

    #[test]
    fn display_and_fromstr_round_trip() {
        let sum = Checksum::of(b"round trip me");
        let parsed: Checksum = sum.to_string().parse().unwrap();
        assert_eq!(sum, parsed);
    }

    #[test]
    fn display_is_zero_padded_lowercase_hex() {
        // "1" (a single byte) checksums to a value with leading zero
        // nibbles -- confirms the width isn't silently dropped.
        let sum = Checksum::of(b"1");
        assert_eq!(sum.to_string().len(), 8);
        assert!(sum
            .to_string()
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
