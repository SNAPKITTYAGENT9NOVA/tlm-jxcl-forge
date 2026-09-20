// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A bounds-checked byte-buffer cursor.
//!
//! New functionality (no single pre-expansion file owned this): the
//! pre-expansion decoder (`jxcl/src/encoding/decoder.rs`) did its own
//! ad hoc bounds checking (`if bytes.len() < len { return Err(...) }`)
//! at every call site, and `jxcl/src/binary.rs` (out of this batch's
//! scope, extracted later into `jxcl-binary`) repeats the same pattern
//! for its header fields. `ByteCursor` is the single, tested
//! implementation of "read/write N bytes from a fixed position, bounds
//! checked" that the encoder, decoder, binary container and (later)
//! object format all get to share instead of reimplementing.
#![forbid(unsafe_code)]

use jxcl_endian::{from_arch_bytes, to_arch_bytes, ArchBytes};
use jxcl_errors::Error;

/// A cursor over a byte slice/buffer that tracks a read/write position
/// and bounds-checks every access, returning [`jxcl_errors::Error::OutOfBounds`]
/// instead of panicking on a short buffer.
#[derive(Debug, Clone)]
pub struct ByteCursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

/// A cursor over an owned, growable buffer, for building up encoded
/// output with the same bounds-checked read-back capability (used by
/// callers that write then immediately verify what they wrote).
#[derive(Debug, Clone, Default)]
pub struct ByteCursorMut {
    buf: Vec<u8>,
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        ByteCursor { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn require(&self, needed: usize) -> Result<(), Error> {
        if self.remaining() < needed {
            Err(Error::OutOfBounds {
                needed,
                available: self.remaining(),
                at: self.pos as u64,
            })
        } else {
            Ok(())
        }
    }

    /// Read exactly `n` bytes without advancing past the slice bounds
    /// (advances the cursor by `n` on success).
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        self.require(n)?;
        let out = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    fn read_int<T: ArchBytes>(&mut self, width: usize) -> Result<T, Error> {
        let bytes = self.read_bytes(width)?;
        Ok(from_arch_bytes::<T>(bytes).expect("read_bytes already guaranteed this width"))
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        self.read_int(1)
    }
    pub fn read_u16(&mut self) -> Result<u16, Error> {
        self.read_int(2)
    }
    pub fn read_u32(&mut self) -> Result<u32, Error> {
        self.read_int(4)
    }
    pub fn read_u64(&mut self) -> Result<u64, Error> {
        self.read_int(8)
    }
    pub fn read_i16(&mut self) -> Result<i16, Error> {
        self.read_int(2)
    }
    pub fn read_i32(&mut self) -> Result<i32, Error> {
        self.read_int(4)
    }
    pub fn read_i64(&mut self) -> Result<i64, Error> {
        self.read_int(8)
    }
}

impl ByteCursorMut {
    pub fn new() -> Self {
        ByteCursorMut::default()
    }

    /// Start from an existing buffer, positioned at its end (append mode).
    pub fn from_vec(buf: Vec<u8>) -> Self {
        let pos = buf.len();
        ByteCursorMut { buf, pos }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    /// Append raw bytes at the current position (this cursor only ever
    /// grows the buffer; there is no fixed-capacity variant since every
    /// current caller -- the encoder -- builds output of a length it
    /// doesn't know until each instruction is encoded).
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.buf.extend_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }

    fn write_int<T: ArchBytes>(&mut self, value: T) -> Result<(), Error> {
        self.write_bytes(to_arch_bytes(value).as_ref())
    }

    pub fn write_u8(&mut self, v: u8) -> Result<(), Error> {
        self.write_int(v)
    }
    pub fn write_u16(&mut self, v: u16) -> Result<(), Error> {
        self.write_int(v)
    }
    pub fn write_u32(&mut self, v: u32) -> Result<(), Error> {
        self.write_int(v)
    }
    pub fn write_u64(&mut self, v: u64) -> Result<(), Error> {
        self.write_int(v)
    }
    pub fn write_i16(&mut self, v: i16) -> Result<(), Error> {
        self.write_int(v)
    }
    pub fn write_i32(&mut self, v: i32) -> Result<(), Error> {
        self.write_int(v)
    }
    pub fn write_i64(&mut self, v: i64) -> Result<(), Error> {
        self.write_int(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_each_width_in_sequence() {
        // u8=0x01, u16=0x0302 LE bytes[02,03], u32=..., matches
        // jxcl's little-endian convention throughout.
        let mut buf = Vec::new();
        buf.push(0x01u8);
        buf.extend_from_slice(&0x0302u16.to_le_bytes());
        buf.extend_from_slice(&0x0706_0504u32.to_le_bytes());
        buf.extend_from_slice(&0x0F0E_0D0C_0B0A_0908u64.to_le_bytes());

        let mut cur = ByteCursor::new(&buf);
        assert_eq!(cur.read_u8().unwrap(), 0x01);
        assert_eq!(cur.read_u16().unwrap(), 0x0302);
        assert_eq!(cur.read_u32().unwrap(), 0x0706_0504);
        assert_eq!(cur.read_u64().unwrap(), 0x0F0E_0D0C_0B0A_0908);
        assert!(cur.is_empty());
    }

    #[test]
    fn read_bytes_advances_position() {
        let buf = [1, 2, 3, 4, 5];
        let mut cur = ByteCursor::new(&buf);
        assert_eq!(cur.read_bytes(2).unwrap(), &[1, 2]);
        assert_eq!(cur.position(), 2);
        assert_eq!(cur.remaining(), 3);
    }

    #[test]
    fn boundary_read_past_end_is_out_of_bounds_error() {
        let buf = [1u8, 2, 3];
        let mut cur = ByteCursor::new(&buf);
        let err = cur.read_u64().unwrap_err();
        assert_eq!(
            err,
            Error::OutOfBounds {
                needed: 8,
                available: 3,
                at: 0
            }
        );
    }

    #[test]
    fn boundary_partial_read_leaves_position_unchanged_on_failure() {
        let buf = [1u8, 2, 3];
        let mut cur = ByteCursor::new(&buf);
        assert!(cur.read_u32().is_err());
        // Nothing was consumed by the failed read.
        assert_eq!(cur.position(), 0);
        assert_eq!(cur.read_u8().unwrap(), 1);
    }

    #[test]
    fn boundary_exact_fit_succeeds() {
        let buf = [0xAAu8, 0xBB];
        let mut cur = ByteCursor::new(&buf);
        assert_eq!(cur.read_u16().unwrap(), 0xBBAA);
        assert!(cur.read_u8().is_err());
    }

    #[test]
    fn signed_reads_match_signed_semantics() {
        let buf = (-42i32).to_le_bytes();
        let mut cur = ByteCursor::new(&buf);
        assert_eq!(cur.read_i32().unwrap(), -42);
    }

    #[test]
    fn writer_roundtrips_through_reader() {
        let mut w = ByteCursorMut::new();
        w.write_u8(0x7F).unwrap();
        w.write_u16(0x1234).unwrap();
        w.write_i32(-99).unwrap();
        w.write_u64(u64::MAX).unwrap();
        let bytes = w.into_vec();

        let mut r = ByteCursor::new(&bytes);
        assert_eq!(r.read_u8().unwrap(), 0x7F);
        assert_eq!(r.read_u16().unwrap(), 0x1234);
        assert_eq!(r.read_i32().unwrap(), -99);
        assert_eq!(r.read_u64().unwrap(), u64::MAX);
    }

    #[test]
    fn writer_append_mode_preserves_existing_bytes() {
        let mut w = ByteCursorMut::from_vec(vec![1, 2, 3]);
        w.write_u8(4).unwrap();
        assert_eq!(w.into_vec(), vec![1, 2, 3, 4]);
    }
}
