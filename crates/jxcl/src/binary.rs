//! The JXCL executable binary format (spec §27).
//!
//! Fixed 56-byte little-endian header, followed by the code section and
//! then the data section (each section's file location is given by the
//! header, so the two are not required to be contiguous *in the file*,
//! though the reference assembler always lays them out back-to-back).
//!
//! When loaded into a machine, code is placed at virtual address `0` and
//! data immediately follows it at virtual address `code_size` — a fixed,
//! deterministic layout with no other placement option, so "where does
//! this binary end up in memory" is never ambiguous (spec §27: "Reject
//! malformed binaries.").
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

use crate::errors::{BinaryFormatError, BinaryFormatErrorKind};
use crate::isa::constants::{ARCHITECTURE_ID, BINARY_MAGIC, BINARY_VERSION};

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
    out.extend_from_slice(&BINARY_VERSION.to_le_bytes());
    out.extend_from_slice(&ARCHITECTURE_ID.to_le_bytes());
    out.extend_from_slice(&entry_point.to_le_bytes());
    out.extend_from_slice(&code_offset.to_le_bytes());
    out.extend_from_slice(&code_size.to_le_bytes());
    out.extend_from_slice(&data_offset.to_le_bytes());
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // FLAGS (reserved)
    out.extend_from_slice(&0u32.to_le_bytes()); // RESERVED
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
    let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    if version != BINARY_VERSION {
        return Err(err(
            BinaryFormatErrorKind::UnsupportedVersion,
            format!("expected version {}, found {}", BINARY_VERSION, version),
        ));
    }
    let architecture = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
    if architecture != ARCHITECTURE_ID {
        return Err(err(
            BinaryFormatErrorKind::UnsupportedArchitecture,
            format!(
                "expected architecture {:#06x}, found {:#06x}",
                ARCHITECTURE_ID, architecture
            ),
        ));
    }
    let entry_point = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let code_offset = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let code_size = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
    let data_offset = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
    let data_size = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
    let flags = u32::from_le_bytes(bytes[48..52].try_into().unwrap());
    let reserved = u32::from_le_bytes(bytes[52..56].try_into().unwrap());
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
}
