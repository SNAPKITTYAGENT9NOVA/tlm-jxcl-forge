//! The memory subsystem (spec §10-§11): a deterministic, bounded,
//! byte-addressable, little-endian flat address space with an explicit
//! permission and alignment model.
//!
//! Architectural decisions (documented per spec's own convention of
//! recording undetermined choices):
//! - **Endianness**: little-endian (spec §10 default).
//! - **Alignment**: 16/32/64-bit accesses must be naturally aligned
//!   (address a multiple of the access width) or they raise
//!   `AlignmentFault`; 8-bit accesses are never misaligned.
//! - **Permissions**: the address space is partitioned into a code
//!   region (readable + executable, not writable) and the remainder
//!   (readable + writable, not executable). This is a minimal but
//!   real permission model, not a stand-in — `LOAD`/`STORE` against
//!   the code region and instruction fetch against the data region
//!   both deterministically fault.

use crate::errors::MemoryFault;

/// A flat, bounded, little-endian address space with a two-region
/// permission split (spec §10, §11).
#[derive(Debug, Clone)]
pub struct Memory {
    data: Vec<u8>,
    /// `[code_start, code_end)`: readable + executable, not writable.
    code_start: u64,
    code_end: u64,
}

/// True iff `[addr, addr+len)` lies entirely within `[start, end)`.
fn in_range(addr: u64, len: u64, start: u64, end: u64) -> bool {
    addr >= start && len <= end.saturating_sub(start) && addr <= end - len
}

/// True iff `[addr, addr+len)` overlaps `[start, end)` at all. Permission
/// checks use this (not `in_range`) so that an access straddling a
/// region boundary is caught rather than silently falling through as
/// "not in the protected region" (a real MMU denies the whole access if
/// any byte of it falls in a protected page).
fn overlaps(addr: u64, len: u64, start: u64, end: u64) -> bool {
    if len == 0 || start == end {
        return false;
    }
    match addr.checked_add(len) {
        Some(range_end) => addr < end && start < range_end,
        None => true, // an access that overflows u64 certainly reaches into `end`
    }
}

impl Memory {
    /// Create a memory of `size` bytes with no executable region (data-only).
    pub fn new(size: u64) -> Self {
        Memory {
            data: vec![0u8; size as usize],
            code_start: 0,
            code_end: 0,
        }
    }

    /// Create a memory of `size` bytes with `[code_start, code_start+code_len)`
    /// marked read+execute (not write); the rest of the space is read+write
    /// (not execute).
    pub fn with_code_region(
        size: u64,
        code_start: u64,
        code_len: u64,
    ) -> Result<Self, MemoryFault> {
        let code_end = code_start
            .checked_add(code_len)
            .ok_or(MemoryFault::InvalidAddress)?;
        if code_end > size {
            return Err(MemoryFault::InvalidAddress);
        }
        Ok(Memory {
            data: vec![0u8; size as usize],
            code_start,
            code_end,
        })
    }

    pub fn len(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn in_bounds(&self, addr: u64, len: u64) -> bool {
        in_range(addr, len, 0, self.len())
    }

    fn in_code_region(&self, addr: u64, len: u64) -> bool {
        in_range(addr, len, self.code_start, self.code_end)
    }

    fn overlaps_code_region(&self, addr: u64, len: u64) -> bool {
        overlaps(addr, len, self.code_start, self.code_end)
    }

    fn check_alignment(addr: u64, width: u64) -> Result<(), MemoryFault> {
        if !addr.is_multiple_of(width) {
            Err(MemoryFault::AlignmentFault)
        } else {
            Ok(())
        }
    }

    /// Read `len` bytes for instruction fetch. Requires the range to lie
    /// entirely within the executable code region.
    pub fn fetch(&self, addr: u64, len: u64) -> Result<&[u8], MemoryFault> {
        if !self.in_bounds(addr, len) {
            return Err(MemoryFault::InvalidAddress);
        }
        if !self.in_code_region(addr, len) {
            return Err(MemoryFault::ExecuteViolation);
        }
        let a = addr as usize;
        let l = len as usize;
        Ok(&self.data[a..a + l])
    }

    fn read_checked(&self, addr: u64, width: u64) -> Result<&[u8], MemoryFault> {
        Self::check_alignment(addr, width)?;
        if !self.in_bounds(addr, width) {
            return Err(MemoryFault::InvalidAddress);
        }
        if self.overlaps_code_region(addr, width) {
            // Data reads never target the code region, even partially
            // (matching most real MMUs' separate I/D read behavior being
            // at least as strict as this).
            return Err(MemoryFault::ReadViolation);
        }
        let a = addr as usize;
        let l = width as usize;
        Ok(&self.data[a..a + l])
    }

    fn write_checked(&mut self, addr: u64, width: u64) -> Result<&mut [u8], MemoryFault> {
        Self::check_alignment(addr, width)?;
        if !self.in_bounds(addr, width) {
            return Err(MemoryFault::InvalidAddress);
        }
        if self.overlaps_code_region(addr, width) {
            return Err(MemoryFault::WriteViolation);
        }
        let a = addr as usize;
        let l = width as usize;
        Ok(&mut self.data[a..a + l])
    }

    pub fn read8(&self, addr: u64) -> Result<u8, MemoryFault> {
        Ok(self.read_checked(addr, 1)?[0])
    }
    pub fn read16(&self, addr: u64) -> Result<u16, MemoryFault> {
        Ok(u16::from_le_bytes(
            self.read_checked(addr, 2)?.try_into().unwrap(),
        ))
    }
    pub fn read32(&self, addr: u64) -> Result<u32, MemoryFault> {
        Ok(u32::from_le_bytes(
            self.read_checked(addr, 4)?.try_into().unwrap(),
        ))
    }
    pub fn read64(&self, addr: u64) -> Result<u64, MemoryFault> {
        Ok(u64::from_le_bytes(
            self.read_checked(addr, 8)?.try_into().unwrap(),
        ))
    }

    pub fn write8(&mut self, addr: u64, value: u8) -> Result<(), MemoryFault> {
        self.write_checked(addr, 1)?[0] = value;
        Ok(())
    }
    pub fn write16(&mut self, addr: u64, value: u16) -> Result<(), MemoryFault> {
        self.write_checked(addr, 2)?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    pub fn write32(&mut self, addr: u64, value: u32) -> Result<(), MemoryFault> {
        self.write_checked(addr, 4)?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    pub fn write64(&mut self, addr: u64, value: u64) -> Result<(), MemoryFault> {
        self.write_checked(addr, 8)?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Loader-only raw write, bypassing the permission model entirely.
    /// Used exactly once, before execution begins, to populate the code
    /// and data sections from a binary (spec §27). Never reachable from
    /// executing instructions.
    pub fn loader_write(&mut self, addr: u64, bytes: &[u8]) -> Result<(), MemoryFault> {
        if !self.in_bounds(addr, bytes.len() as u64) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr as usize;
        self.data[a..a + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// Debug-only, permission-bypassing peek at raw bytes, used by the
    /// trace/debugger (spec §30) to render an instruction's text even
    /// when stepping has already halted/faulted. Never reachable from
    /// executing instructions — only `debugger` and the CLI call it.
    pub fn peek(&self, addr: u64, len: u64) -> Option<&[u8]> {
        if !self.in_bounds(addr, len) {
            return None;
        }
        let a = addr as usize;
        let l = len as usize;
        Some(&self.data[a..a + l])
    }

    /// Full snapshot of memory contents, for `MachineState::snapshot` (spec §12).
    pub fn snapshot(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn restore(&mut self, bytes: Vec<u8>) {
        self.data = bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_roundtrip() {
        let mut m = Memory::new(64);
        m.write64(8, 0xDEADBEEFCAFEBABE).unwrap();
        assert_eq!(m.read64(8).unwrap(), 0xDEADBEEFCAFEBABE);
    }

    #[test]
    fn little_endian_byte_order() {
        let mut m = Memory::new(16);
        m.write32(0, 0x11223344).unwrap();
        assert_eq!(m.read8(0).unwrap(), 0x44);
        assert_eq!(m.read8(3).unwrap(), 0x11);
    }

    #[test]
    fn alignment_fault_on_misaligned_access() {
        let m = Memory::new(16);
        assert_eq!(m.read32(1).unwrap_err(), MemoryFault::AlignmentFault);
    }

    #[test]
    fn out_of_bounds_is_invalid_address() {
        let m = Memory::new(16);
        assert_eq!(m.read64(16).unwrap_err(), MemoryFault::InvalidAddress);
    }

    #[test]
    fn code_region_is_not_writable() {
        let mut m = Memory::with_code_region(64, 0, 16).unwrap();
        assert_eq!(m.write8(0, 1).unwrap_err(), MemoryFault::WriteViolation);
        assert_eq!(m.read8(0).unwrap_err(), MemoryFault::ReadViolation);
    }

    #[test]
    fn straddling_access_into_code_region_is_still_denied() {
        // Code region is [0, 12); an 8-aligned write of width 8 at
        // address 8 spans bytes 8..16, only partially overlapping the
        // code region (8..12 in, 12..16 out). It must still be denied —
        // a real MMU would deny the whole access, not just the covered part.
        let mut m = Memory::with_code_region(64, 0, 12).unwrap();
        assert_eq!(m.write64(8, 1).unwrap_err(), MemoryFault::WriteViolation);
        assert_eq!(m.read64(8).unwrap_err(), MemoryFault::ReadViolation);
    }

    #[test]
    fn data_region_is_not_executable() {
        let m = Memory::with_code_region(64, 0, 16).unwrap();
        assert_eq!(m.fetch(16, 1).unwrap_err(), MemoryFault::ExecuteViolation);
        assert!(m.fetch(0, 1).is_ok());
    }
}
