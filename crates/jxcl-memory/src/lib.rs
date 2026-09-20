// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Raw byte-addressable memory storage and bounds-checked byte-level access.
//!
//! `Memory` is the only place raw bytes are stored (`docs/crates.toml`).
//! Region/permission concepts (code vs. data, read/write/execute) are
//! layered on top of this crate by `jxcl-address-space`, which owns that
//! separate invariant; this crate only ever answers "is this byte range
//! inside the buffer" and "is this access naturally aligned".
//!
//! Behavior is grounded in `crates/jxcl/src/memory.rs`'s bounds/alignment
//! rules and little-endian byte order (`jxcl-endian`), minus that file's
//! code/data permission split, which now belongs to `jxcl-address-space`.
#![forbid(unsafe_code)]

use jxcl_endian::{from_arch_bytes, to_arch_bytes};
use jxcl_errors::MemoryFault;
use jxcl_types::Address;

/// A flat, bounded, byte-addressable memory buffer.
#[derive(Debug, Clone)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    /// Create a `size`-byte memory, zero-initialized.
    pub fn new(size: u64) -> Self {
        Memory {
            data: vec![0u8; size as usize],
        }
    }

    /// Create a memory pre-populated from `bytes` (its length is the memory size).
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Memory { data: bytes }
    }

    /// Total size in bytes.
    pub fn len(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn in_bounds(&self, addr: Address, len: u64) -> bool {
        match addr.get().checked_add(len) {
            Some(end) => end <= self.len(),
            None => false,
        }
    }

    fn check_alignment(addr: Address, width: u64) -> Result<(), MemoryFault> {
        if addr.get().is_multiple_of(width) {
            Ok(())
        } else {
            Err(MemoryFault::AlignmentFault)
        }
    }

    /// Read `len` raw bytes, no alignment requirement (used for
    /// instruction fetch and other byte-granular access).
    pub fn read_bytes(&self, addr: Address, len: u64) -> Result<&[u8], MemoryFault> {
        if !self.in_bounds(addr, len) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr.get() as usize;
        let l = len as usize;
        Ok(&self.data[a..a + l])
    }

    /// Write `bytes` at `addr`, no alignment requirement.
    pub fn write_bytes(&mut self, addr: Address, bytes: &[u8]) -> Result<(), MemoryFault> {
        if !self.in_bounds(addr, bytes.len() as u64) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr.get() as usize;
        self.data[a..a + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    fn read_checked(&self, addr: Address, width: u64) -> Result<&[u8], MemoryFault> {
        Self::check_alignment(addr, width)?;
        self.read_bytes(addr, width)
    }

    fn write_checked(&mut self, addr: Address, width: u64) -> Result<&mut [u8], MemoryFault> {
        Self::check_alignment(addr, width)?;
        if !self.in_bounds(addr, width) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr.get() as usize;
        let l = width as usize;
        Ok(&mut self.data[a..a + l])
    }

    pub fn read8(&self, addr: Address) -> Result<u8, MemoryFault> {
        Ok(self.read_checked(addr, 1)?[0])
    }
    pub fn read16(&self, addr: Address) -> Result<u16, MemoryFault> {
        Ok(from_arch_bytes(self.read_checked(addr, 2)?).expect("width already checked"))
    }
    pub fn read32(&self, addr: Address) -> Result<u32, MemoryFault> {
        Ok(from_arch_bytes(self.read_checked(addr, 4)?).expect("width already checked"))
    }
    pub fn read64(&self, addr: Address) -> Result<u64, MemoryFault> {
        Ok(from_arch_bytes(self.read_checked(addr, 8)?).expect("width already checked"))
    }

    pub fn write8(&mut self, addr: Address, value: u8) -> Result<(), MemoryFault> {
        self.write_checked(addr, 1)?[0] = value;
        Ok(())
    }
    pub fn write16(&mut self, addr: Address, value: u16) -> Result<(), MemoryFault> {
        self.write_checked(addr, 2)?
            .copy_from_slice(to_arch_bytes(value).as_ref());
        Ok(())
    }
    pub fn write32(&mut self, addr: Address, value: u32) -> Result<(), MemoryFault> {
        self.write_checked(addr, 4)?
            .copy_from_slice(to_arch_bytes(value).as_ref());
        Ok(())
    }
    pub fn write64(&mut self, addr: Address, value: u64) -> Result<(), MemoryFault> {
        self.write_checked(addr, 8)?
            .copy_from_slice(to_arch_bytes(value).as_ref());
        Ok(())
    }

    /// Full snapshot of memory contents (for machine-state snapshot/restore).
    pub fn snapshot(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Restore memory contents from a previous [`Memory::snapshot`]. The
    /// restored buffer's length becomes this memory's new length.
    pub fn restore(&mut self, bytes: Vec<u8>) {
        self.data = bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(v: u64) -> Address {
        Address::new(v)
    }

    #[test]
    fn read_write_roundtrip() {
        let mut m = Memory::new(64);
        m.write64(addr(8), 0xDEAD_BEEF_CAFE_BABE).unwrap();
        assert_eq!(m.read64(addr(8)).unwrap(), 0xDEAD_BEEF_CAFE_BABE);
    }

    #[test]
    fn little_endian_byte_order() {
        let mut m = Memory::new(16);
        m.write32(addr(0), 0x1122_3344).unwrap();
        assert_eq!(m.read8(addr(0)).unwrap(), 0x44);
        assert_eq!(m.read8(addr(3)).unwrap(), 0x11);
    }

    #[test]
    fn alignment_fault_on_misaligned_access() {
        let m = Memory::new(16);
        assert_eq!(m.read32(addr(1)).unwrap_err(), MemoryFault::AlignmentFault);
    }

    #[test]
    fn out_of_bounds_is_invalid_address() {
        let m = Memory::new(16);
        assert_eq!(m.read64(addr(16)).unwrap_err(), MemoryFault::InvalidAddress);
    }

    #[test]
    fn read_bytes_is_alignment_free() {
        let mut m = Memory::new(16);
        m.write_bytes(addr(1), &[1, 2, 3]).unwrap();
        assert_eq!(m.read_bytes(addr(1), 3).unwrap(), &[1, 2, 3]);
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut m = Memory::new(8);
        m.write64(addr(0), 42).unwrap();
        let snap = m.snapshot();
        let mut m2 = Memory::new(8);
        m2.restore(snap);
        assert_eq!(m2.read64(addr(0)).unwrap(), 42);
    }

    #[test]
    fn address_overflow_is_out_of_bounds_not_a_panic() {
        let m = Memory::new(16);
        assert_eq!(
            m.read_bytes(Address::new(u64::MAX), 8).unwrap_err(),
            MemoryFault::InvalidAddress
        );
    }
}
