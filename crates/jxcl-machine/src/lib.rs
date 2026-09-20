// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Architectural state: the register file, flags, and program counter as one cohesive Machine struct.
//!
//! Owns: The Machine struct -- the single authoritative representation of architectural state.
//! Extracted from `jxcl/src/machine.rs`.
//!
//! The complete, serializable architectural state of one JXCL machine, including
//! registers, memory, halt/fault state, and a cycle counter (spec §12).
#![forbid(unsafe_code)]

use jxcl_errors::ExecutionFault;
use jxcl_memory::Memory;
use jxcl_registers::RegisterFile;
use jxcl_types::RegisterIndex;

/// A request for host attention, raised by SYS/TRAP (spec §26).
/// The base ISA never acts on this itself — it is purely data for whatever
/// host integration chooses to interpret it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Syscall(u16),
    Trap(u16),
}

/// The complete architectural machine state: register file, memory,
/// halt/fault state and a cycle counter (spec §12).
#[derive(Debug, Clone)]
pub struct Machine {
    pub registers: RegisterFile,
    pub memory: Memory,
    pub halted: bool,
    pub fault: Option<ExecutionFault>,
    pub cycle_count: u64,
    /// Set by SYS/TRAP (spec §26); the host-side CLI/debugger observes
    /// this after execution returns rather than JXCL performing any
    /// implicit host interaction itself.
    pub pending_signal: Option<Signal>,
}

/// A fully serializable snapshot of a `Machine` (spec §12:
/// "The complete state must be serializable.").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSnapshot {
    pub general_registers: Vec<u64>,
    pub pc: u64,
    pub sp: u64,
    pub fp: u64,
    pub flags: u64,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub fault: Option<ExecutionFault>,
    pub cycle_count: u64,
}

impl Machine {
    /// Construct a fresh machine over `memory`. SP is initialized to
    /// `memory.len()` — one past the last valid byte — so that the first
    /// `PUSH` (which decrements SP before writing, per the downward-growing
    /// stack convention, spec §15) lands at the highest usable address.
    pub fn new(memory: Memory) -> Self {
        let sp = memory.len() as u64;
        let mut machine = Machine {
            registers: RegisterFile::new(),
            memory,
            halted: false,
            fault: None,
            cycle_count: 0,
            pending_signal: None,
        };
        machine.registers.sp = sp;
        machine
    }

    /// Take a serializable snapshot of the current machine state.
    pub fn snapshot(&self) -> MachineSnapshot {
        let general_registers = self
            .registers
            .general_registers()
            .iter()
            .copied()
            .collect();

        MachineSnapshot {
            general_registers,
            pc: self.registers.pc,
            sp: self.registers.sp,
            fp: self.registers.fp,
            flags: self.registers.flags,
            memory: self.memory.snapshot(),
            halted: self.halted,
            fault: self.fault,
            cycle_count: self.cycle_count,
        }
    }

    /// Restore the machine state from a snapshot.
    pub fn restore(&mut self, snap: MachineSnapshot) {
        for (i, val) in snap.general_registers.iter().enumerate() {
            if i < self.registers.general_registers().len() {
                let _ = self.registers.write(RegisterIndex::new(i as u8), *val);
            }
        }
        self.registers.pc = snap.pc;
        self.registers.sp = snap.sp;
        self.registers.fp = snap.fp;
        self.registers.flags = snap.flags;
        self.memory.restore(snap.memory);
        self.halted = snap.halted;
        self.fault = snap.fault;
        self.cycle_count = snap.cycle_count;
        self.pending_signal = None;
    }

    /// Check if the machine is halted.
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Check if the machine has encountered a fault.
    pub fn has_fault(&self) -> bool {
        self.fault.is_some()
    }

    /// Halt the machine.
    pub fn halt(&mut self) {
        self.halted = true;
    }

    /// Set a fault condition.
    pub fn set_fault(&mut self, fault: ExecutionFault) {
        self.fault = Some(fault);
    }

    /// Increment the cycle counter.
    pub fn add_cycles(&mut self, cycles: u64) {
        self.cycle_count = self.cycle_count.wrapping_add(cycles);
    }

    /// Set a pending signal.
    pub fn set_signal(&mut self, signal: Signal) {
        self.pending_signal = Some(signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_creation() {
        let mem = Memory::new(1024);
        let machine = Machine::new(mem);
        assert!(!machine.halted);
        assert!(machine.fault.is_none());
        assert_eq!(machine.cycle_count, 0);
    }

    #[test]
    fn machine_sp_initialization() {
        let mem = Memory::new(512);
        let machine = Machine::new(mem);
        assert_eq!(machine.registers.sp, 512);
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mem = Memory::new(256);
        let mut machine = Machine::new(mem);

        // Modify machine state
        machine.registers.write(RegisterIndex::new(1), 0xABCD).unwrap();
        machine.registers.pc = 4;
        machine.cycle_count = 7;

        // Take snapshot
        let snap = machine.snapshot();

        // Restore to a new machine
        let mut machine2 = Machine::new(Memory::new(256));
        machine2.restore(snap.clone());

        // Verify the snapshot is identical
        assert_eq!(machine2.snapshot(), snap);
    }

    #[test]
    fn machine_halt() {
        let mem = Memory::new(256);
        let mut machine = Machine::new(mem);
        assert!(!machine.is_halted());

        machine.halt();
        assert!(machine.is_halted());
    }

    #[test]
    fn machine_fault() {
        let mem = Memory::new(256);
        let mut machine = Machine::new(mem);
        assert!(!machine.has_fault());

        machine.set_fault(ExecutionFault::InvalidRegister);
        assert!(machine.has_fault());
    }

    #[test]
    fn machine_cycles() {
        let mem = Memory::new(256);
        let mut machine = Machine::new(mem);
        assert_eq!(machine.cycle_count, 0);

        machine.add_cycles(5);
        assert_eq!(machine.cycle_count, 5);

        machine.add_cycles(3);
        assert_eq!(machine.cycle_count, 8);
    }

    #[test]
    fn machine_signal() {
        let mem = Memory::new(256);
        let mut machine = Machine::new(mem);
        assert!(machine.pending_signal.is_none());

        machine.set_signal(Signal::Syscall(42));
        assert_eq!(machine.pending_signal, Some(Signal::Syscall(42)));
    }
}
