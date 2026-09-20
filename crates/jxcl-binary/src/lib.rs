// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The final linked-executable container format: header, version,
//! code/data sections.
//!
//! Extracted, near verbatim, from `jxcl/src/binary.rs`. Fixed 56-byte
//! little-endian header, followed by the code section and then the data
//! section (each section's file location is given by the header, so the
//! two are not required to be contiguous *in the file*, though every
//! writer in this workspace lays them out back-to-back).
//!
//! When loaded into a machine, code is placed at virtual address `0` and
//! data immediately follows it at virtual address `code_size` -- a
//! fixed, deterministic layout with no other placement option, so
//! "where does this binary end up in memory" is never ambiguous
//! (`jxcl-loader` is the consumer of that convention).
//!
//! ```text
//! offset  size  field
//! 0       4     MAGIC        b"JXCL"
//! 4       2     VERSION      (little-endian u16)
//! 6       2     ARCHITECTURE (little-endian u16)
//! 8       8     ENTRY_POINT  (little-endian u64, virtual address)
//! 16      8     CODE_OFFSET  (little-endian u64, file offset)
//! 24      8     CODE_SIZE    (little-endian u64, bytes)
//! 32      8     DATA_OFFSET  (little-endian u64, file offset)
//! 40      8     DATA_SIZE    (little-endian u64, bytes)
//! 48      4     FLAGS        (little-endian u32, reserved, must be 0)
//! 52      4     RESERVED     (must be 0)
//! ```
//!
//! ## Deviation from `docs/crates.toml`
//!
//! The registry lists `jxcl-isa-versioning` as a dependency, for the
//! `VERSION`/`ARCHITECTURE` compatibility check. At the time this crate
//! was implemented, `jxcl-isa-versioning` was still a scaffolded
//! placeholder with no public items (and it is not one of the crates
//! this batch's sibling agents are landing concurrently either), so the
//! version/architecture check is implemented directly against
//! `jxcl-constants::{BINARY_VERSION, ARCHITECTURE_ID}` -- exactly what
//! the real pre-expansion `jxcl/src/binary.rs` does. Once
//! `jxcl-isa-versioning` lands with its documented `IsaVersion`/
//! `is_compatible_with` API, [`parse`]'s version/architecture check
//! should delegate to it instead of comparing the raw constants
//! in-line; the externally observable behavior (reject any binary whose
//! version or architecture byte doesn't match) would not change.
#![forbid(unsafe_code)]

use jxcl_constants::{ARCHITECTURE_ID, BINARY_MAGIC, BINARY_VERSION};
use jxcl_endian::{from_arch_bytes, to_arch_bytes};
use jxcl_errors::{BinaryFormatError, BinaryFormatErrorKind};

pub const HEADER_SIZE: usize = 56;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryHeader {
    pub version: u16,
    pub architecture: u16,
    pub entry_point: u64,
    pub code_offset: u64,
    pub code_size: u64,
    pub data_offset: u64,
    pub data_size: u64,
    pub flags: u32,
}

/// The final linked executable container: a header plus owned code/data
/// byte buffers (matches `docs/crates.toml`'s `public_api` entry
/// `BinaryContainer`; the pre-expansion source's borrowed `Program<'a>`
/// is kept alongside as [`Program`] for callers that only need to parse
/// a byte slice without taking ownership of a copy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryContainer {
    pub entry_point: u64,
    pub code: Vec<u8>,
    pub data: Vec<u8>,
}

impl BinaryContainer {
    pub fn new(entry_point: u64, code: Vec<u8>, data: Vec<u8>) -> Self {
        BinaryContainer {
            entry_point,
            code,
            data,
        }
    }

    /// Serialize to a complete `.jxc` file (see [`write`]).
    pub fn to_bytes(&self) -> Vec<u8> {
        write(self.entry_point, &self.code, &self.data)
    }

    /// Parse and validate a `.jxc` file into an owned [`BinaryContainer`]
    /// (see [`parse`]).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BinaryFormatError> {
        let program = parse(bytes)?;
        Ok(BinaryContainer {
            entry_point: program.header.entry_point,
            code: program.code.to_vec(),
            data: program.data.to_vec(),
        })
    }
}

/// A fully parsed JXCL executable: header plus borrowed code/data slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Program<'a> {
    pub header: BinaryHeader,
    pub code: &'a [u8],
    pub data: &'a [u8],
}

/// Serialize `code` and `data` into a complete `.jxc` file, laid out
/// back-to-back immediately after the header.
pub fn write(entry_point: u64, code: &[u8], data: &[u8]) -> Vec<u8> {
    let code_offset = HEADER_SIZE as u64;
    let code_size = code.len() as u64;
    let data_offset = code_offset + code_size;
    let data_size = data.len() as u64;

    let mut out = Vec::with_capacity(HEADER_SIZE + code.len() + data.len());
    out.extend_from_slice(&BINARY_MAGIC);
    out.extend_from_slice(&to_arch_bytes(BINARY_VERSION));
    out.extend_from_slice(&to_arch_bytes(ARCHITECTURE_ID));
    out.extend_from_slice(&to_arch_bytes(entry_point));
    out.extend_from_slice(&to_arch_bytes(code_offset));
    out.extend_from_slice(&to_arch_bytes(code_size));
    out.extend_from_slice(&to_arch_bytes(data_offset));
    out.extend_from_slice(&to_arch_bytes(data_size));
    out.extend_from_slice(&to_arch_bytes(0u32)); // FLAGS (reserved)
    out.extend_from_slice(&to_arch_bytes(0u32)); // RESERVED
    debug_assert_eq!(out.len(), HEADER_SIZE);
    out.extend_from_slice(code);
    out.extend_from_slice(data);
    out
}

fn err(kind: BinaryFormatErrorKind, reason: impl Into<String>) -> BinaryFormatError {
    BinaryFormatError {
        kind,
        reason: reason.into(),
    }
}

/// Parse and validate a `.jxc` file's header, returning borrowed views of
/// its code and data sections. Every field is range-checked against the
/// file's actual length before any slice is taken.
pub fn parse(bytes: &[u8]) -> Result<Program<'_>, BinaryFormatError> {
    if bytes.len() < HEADER_SIZE {
        return Err(err(
            BinaryFormatErrorKind::TooShort,
            "file shorter than the 56-byte header",
        ));
    }
    if bytes[0..4] != BINARY_MAGIC {
        return Err(err(BinaryFormatErrorKind::BadMagic, "missing JXCL magic"));
    }
    let version: u16 = from_arch_bytes(&bytes[4..6]).expect("length checked above");
    if version != BINARY_VERSION {
        return Err(err(
            BinaryFormatErrorKind::UnsupportedVersion,
            format!("expected version {}, found {}", BINARY_VERSION, version),
        ));
    }
    let architecture: u16 = from_arch_bytes(&bytes[6..8]).expect("length checked above");
    if architecture != ARCHITECTURE_ID {
        return Err(err(
            BinaryFormatErrorKind::UnsupportedArchitecture,
            format!(
                "expected architecture {:#06x}, found {:#06x}",
                ARCHITECTURE_ID, architecture
            ),
        ));
    }
    let entry_point: u64 = from_arch_bytes(&bytes[8..16]).expect("length checked above");
    let code_offset: u64 = from_arch_bytes(&bytes[16..24]).expect("length checked above");
    let code_size: u64 = from_arch_bytes(&bytes[24..32]).expect("length checked above");
    let data_offset: u64 = from_arch_bytes(&bytes[32..40]).expect("length checked above");
    let data_size: u64 = from_arch_bytes(&bytes[40..48]).expect("length checked above");
    let flags: u32 = from_arch_bytes(&bytes[48..52]).expect("length checked above");
    let reserved: u32 = from_arch_bytes(&bytes[52..56]).expect("length checked above");
    if reserved != 0 {
        return Err(err(
            BinaryFormatErrorKind::BadMagic,
            "reserved header field must be zero",
        ));
    }

    let code_end = code_offset.checked_add(code_size).ok_or_else(|| {
        err(
            BinaryFormatErrorKind::SizeOutOfRange,
            "code section overflows u64",
        )
    })?;
    if code_end > bytes.len() as u64 {
        return Err(err(
            BinaryFormatErrorKind::OffsetOutOfRange,
            "code section exceeds file length",
        ));
    }
    let data_end = data_offset.checked_add(data_size).ok_or_else(|| {
        err(
            BinaryFormatErrorKind::SizeOutOfRange,
            "data section overflows u64",
        )
    })?;
    if data_end > bytes.len() as u64 {
        return Err(err(
            BinaryFormatErrorKind::OffsetOutOfRange,
            "data section exceeds file length",
        ));
    }

    let header = BinaryHeader {
        version,
        architecture,
        entry_point,
        code_offset,
        code_size,
        data_offset,
        data_size,
        flags,
    };
    let code = &bytes[code_offset as usize..code_end as usize];
    let data = &bytes[data_offset as usize..data_end as usize];
    Ok(Program { header, code, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_parse_roundtrip() {
        let code = vec![0x60]; // HALT
        let data = vec![1, 2, 3, 4];
        let bin = write(0, &code, &data);
        let parsed = parse(&bin).unwrap();
        assert_eq!(parsed.code, &code[..]);
        assert_eq!(parsed.data, &data[..]);
        assert_eq!(parsed.header.entry_point, 0);
    }

    #[test]
    fn container_to_bytes_from_bytes_roundtrip() {
        let container = BinaryContainer::new(4, vec![0x60, 0x60, 0x60, 0x60], vec![9, 9]);
        let bytes = container.to_bytes();
        let restored = BinaryContainer::from_bytes(&bytes).unwrap();
        assert_eq!(container, restored);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bin = write(0, &[0x60], &[]);
        bin[0] = b'X';
        assert_eq!(
            parse(&bin).unwrap_err().kind,
            BinaryFormatErrorKind::BadMagic
        );
    }

    #[test]
    fn rejects_truncated_header() {
        let bin = vec![0u8; 10];
        assert_eq!(
            parse(&bin).unwrap_err().kind,
            BinaryFormatErrorKind::TooShort
        );
    }

    #[test]
    fn rejects_oversized_code_section() {
        let mut bin = write(0, &[0x60], &[]);
        // Corrupt CODE_SIZE to claim far more bytes than the file has.
        bin[24..32].copy_from_slice(&(1_000_000u64).to_le_bytes());
        assert_eq!(
            parse(&bin).unwrap_err().kind,
            BinaryFormatErrorKind::OffsetOutOfRange
        );
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut bin = write(0, &[0x60], &[]);
        bin[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            parse(&bin).unwrap_err().kind,
            BinaryFormatErrorKind::UnsupportedVersion
        );
    }

    #[test]
    fn rejects_unsupported_architecture() {
        let mut bin = write(0, &[0x60], &[]);
        bin[6..8].copy_from_slice(&0xFFFFu16.to_le_bytes());
        assert_eq!(
            parse(&bin).unwrap_err().kind,
            BinaryFormatErrorKind::UnsupportedArchitecture
        );
    }

    #[test]
    fn rejects_nonzero_reserved_field() {
        let mut bin = write(0, &[0x60], &[]);
        bin[52..56].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            parse(&bin).unwrap_err().kind,
            BinaryFormatErrorKind::BadMagic
        );
    }

    #[test]
    fn boundary_exact_length_file_parses() {
        let bin = write(0, &[], &[]);
        assert_eq!(bin.len(), HEADER_SIZE);
        let parsed = parse(&bin).unwrap();
        assert!(parsed.code.is_empty());
        assert!(parsed.data.is_empty());
    }

    #[test]
    fn golden_header_layout_matches_spec() {
        // A golden byte-for-byte check of the header layout, so any
        // accidental field reordering is caught immediately.
        let bin = write(0x10, &[0xAA], &[0xBB, 0xCC]);
        assert_eq!(&bin[0..4], b"JXCL");
        assert_eq!(&bin[4..6], &BINARY_VERSION.to_le_bytes());
        assert_eq!(&bin[6..8], &ARCHITECTURE_ID.to_le_bytes());
        assert_eq!(&bin[8..16], &0x10u64.to_le_bytes());
        assert_eq!(&bin[16..24], &(HEADER_SIZE as u64).to_le_bytes());
        assert_eq!(&bin[24..32], &1u64.to_le_bytes());
        assert_eq!(&bin[32..40], &(HEADER_SIZE as u64 + 1).to_le_bytes());
        assert_eq!(&bin[40..48], &2u64.to_le_bytes());
        assert_eq!(&bin[48..52], &0u32.to_le_bytes());
        assert_eq!(&bin[52..56], &0u32.to_le_bytes());
        assert_eq!(bin[56], 0xAA);
        assert_eq!(&bin[57..59], &[0xBB, 0xCC]);
    }
}
