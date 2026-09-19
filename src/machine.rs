//! `MachineState` (spec §12): the complete, serializable architectural
//! state of one JXCL machine.

use crate::errors::ExecutionFault;
use crate::isa::constants::NUM_GP_REGISTERS;
use crate::isa::registers::RegisterFile;
use crate::memory::Memory;

/// The complete architectural machine state: register file, memory,
/// halt/fault state and a cycle counter (spec §12).
#[derive(Debug, Clone)]
pub struct MachineState {
    pub registers: RegisterFile,
    pub memory: Memory,
    pub halted: bool,
    pub fault: Option<ExecutionFault>,
    pub cycle_count: u64,
    /// Set by SYS/TRAP (spec §26); the host-side CLI/debugger observes
    /// this after `step()` returns rather than JXCL performing any
    /// implicit host interaction itself.
    pub pending_signal: Option<Signal>,
}

/// A request for host attention, raised by SYS/TRAP (spec §26). The base
/// ISA never acts on this itself — it is purely data for whatever host
/// integration chooses to interpret it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Syscall(u16),
    Trap(u16),
}

/// A fully serializable snapshot of a `MachineState` (spec §12:
/// "The complete state must be serializable.").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSnapshot {
    pub general_registers: [u64; NUM_GP_REGISTERS],
    pub pc: u64,
    pub sp: u64,
    pub fp: u64,
    pub flags: u64,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub fault: Option<ExecutionFault>,
    pub cycle_count: u64,
}

impl MachineState {
    /// Construct a fresh machine over `memory`. SP is initialized to
    /// `memory.len()` — one past the last valid byte — so that the first
    /// `PUSH` (which decrements SP before writing, per the downward-growing
    /// stack convention, spec §15) lands at the highest usable address.
    pub fn new(memory: Memory) -> Self {
        let sp = memory.len();
        MachineState {
            registers: RegisterFile::new(),
            memory,
            halted: false,
            fault: None,
            cycle_count: 0,
            pending_signal: None,
        }
        .with_sp(sp)
    }

    fn with_sp(mut self, sp: u64) -> Self {
        self.registers.sp = sp;
        self
    }

    pub fn snapshot(&self) -> MachineSnapshot {
        MachineSnapshot {
            general_registers: *self.registers.general_registers(),
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

    pub fn restore(&mut self, snap: MachineSnapshot) {
        self.registers.set_general_registers(snap.general_registers);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_restore_roundtrip() {
        let mem = Memory::new(64);
        let mut m = MachineState::new(mem);
        m.registers.write(1, 0xABCD).unwrap();
        m.registers.pc = 4;
        m.cycle_count = 7;
        let snap = m.snapshot();

        let mut m2 = MachineState::new(Memory::new(64));
        m2.restore(snap.clone());
        assert_eq!(m2.snapshot(), snap);
    }
}
