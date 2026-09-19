//! Architectural state for this crate's embedded machine: register file
//! and a flat, bounded, little-endian memory, ported (in simplified
//! form -- no code/data permission split, which is `jxcl-memory-map`'s
//! job once it lands) from `crates/jxcl/src/machine.rs` and
//! `crates/jxcl/src/memory.rs`.

use crate::isa::NUM_GP_REGISTERS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFault {
    InvalidAddress,
    AlignmentFault,
}

impl std::fmt::Display for MemoryFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for MemoryFault {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionFault {
    InvalidOpcode,
    InvalidRegister,
    TruncatedInstruction,
    DivideByZero,
    StackFault,
    Memory(MemoryFault),
}

impl std::fmt::Display for ExecutionFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for ExecutionFault {}

impl From<MemoryFault> for ExecutionFault {
    fn from(m: MemoryFault) -> Self {
        ExecutionFault::Memory(m)
    }
}

/// A flat, bounded, little-endian byte-addressable memory.
#[derive(Debug, Clone)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new(size: u64) -> Self {
        Memory {
            data: vec![0u8; size as usize],
        }
    }

    pub fn len(&self) -> u64 {
        self.data.len() as u64
    }

    fn in_bounds(&self, addr: u64, len: u64) -> bool {
        match addr.checked_add(len) {
            Some(end) => end <= self.len(),
            None => false,
        }
    }

    fn check(addr: u64, width: u64) -> Result<(), MemoryFault> {
        if !addr.is_multiple_of(width) {
            Err(MemoryFault::AlignmentFault)
        } else {
            Ok(())
        }
    }

    pub fn read64(&self, addr: u64) -> Result<u64, MemoryFault> {
        Self::check(addr, 8)?;
        if !self.in_bounds(addr, 8) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr as usize;
        Ok(u64::from_le_bytes(self.data[a..a + 8].try_into().unwrap()))
    }

    pub fn write64(&mut self, addr: u64, value: u64) -> Result<(), MemoryFault> {
        Self::check(addr, 8)?;
        if !self.in_bounds(addr, 8) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr as usize;
        self.data[a..a + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Fetch up to `len` bytes for instruction decode; used only with
    /// `len` bounded by the longest format in `isa::Format`.
    pub fn fetch(&self, addr: u64, len: u64) -> Result<&[u8], MemoryFault> {
        if !self.in_bounds(addr, len) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr as usize;
        Ok(&self.data[a..a + len as usize])
    }

    /// Loader-only raw write, bypassing alignment checks. Used exactly
    /// once, at construction, to place the initial program image.
    pub fn load(&mut self, addr: u64, bytes: &[u8]) -> Result<(), MemoryFault> {
        if !self.in_bounds(addr, bytes.len() as u64) {
            return Err(MemoryFault::InvalidAddress);
        }
        let a = addr as usize;
        self.data[a..a + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
}

/// The complete architectural state of one embedded machine.
#[derive(Debug, Clone)]
pub struct MachineState {
    pub registers: [u64; NUM_GP_REGISTERS],
    pub pc: u64,
    pub sp: u64,
    pub flags: u64,
    pub memory: Memory,
    pub halted: bool,
    pub fault: Option<ExecutionFault>,
    pub cycle_count: u64,
}

impl MachineState {
    /// R0 is hardwired to zero, matching `crates/jxcl`'s architectural
    /// decision (spec §5): reads always yield 0, writes are discarded.
    pub fn read_reg(&self, id: u8) -> Result<u64, ExecutionFault> {
        if (id as usize) >= NUM_GP_REGISTERS {
            return Err(ExecutionFault::InvalidRegister);
        }
        if id == 0 {
            return Ok(0);
        }
        Ok(self.registers[id as usize])
    }

    pub fn write_reg(&mut self, id: u8, value: u64) -> Result<(), ExecutionFault> {
        if (id as usize) >= NUM_GP_REGISTERS {
            return Err(ExecutionFault::InvalidRegister);
        }
        if id != 0 {
            self.registers[id as usize] = value;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r0_reads_zero_and_discards_writes() {
        let mut m = MachineState {
            registers: [0; NUM_GP_REGISTERS],
            pc: 0,
            sp: 0,
            flags: 0,
            memory: Memory::new(16),
            halted: false,
            fault: None,
            cycle_count: 0,
        };
        m.write_reg(0, 0xFF).unwrap();
        assert_eq!(m.read_reg(0).unwrap(), 0);
    }

    #[test]
    fn memory_roundtrip_and_alignment() {
        let mut mem = Memory::new(32);
        assert_eq!(mem.len(), 32);
        mem.write64(8, 0xDEAD_BEEF).unwrap();
        assert_eq!(mem.read64(8).unwrap(), 0xDEAD_BEEF);
        assert_eq!(mem.read64(1).unwrap_err(), MemoryFault::AlignmentFault);
        assert_eq!(mem.read64(32).unwrap_err(), MemoryFault::InvalidAddress);
    }
}
