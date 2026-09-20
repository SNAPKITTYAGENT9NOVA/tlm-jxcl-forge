// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Machine state snapshot/restore and the determinism property-test
//! harness (spec source: `jxcl/tests/property_tests.rs`'s
//! `machine_snapshot_restore_is_lossless_under_random_mutation` test,
//! generalized into a real, reusable type + trait).
//!
//! This crate owns exactly two things:
//!
//! - [`Snapshot`]: a fully-owned, comparable copy of a machine's
//!   architectural state (general registers, PC/SP/FP, flags, memory,
//!   cycle count). It never borrows from the machine it was taken from,
//!   so it can be stashed, compared, or replayed independently of the
//!   machine's lifetime.
//! - [`SnapshotRestore`]: the trait any steppable machine implements to
//!   plug into snapshot/restore and the determinism property tests
//!   below, without this crate needing to depend on that machine type's
//!   concrete representation.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` lists this crate's dependencies as `jxcl-machine`
//! and `jxcl-memory` (both still scaffolded placeholders as of this
//! batch — see this crate's README for the exact landing plan). Rather
//! than block on those crates landing, `SnapshotRestore` is defined
//! generically here so it is immediately real and testable: once
//! `jxcl-machine::Machine` (or `jxcl::machine::MachineState`) exists,
//! implementing `SnapshotRestore for Machine` there (or in a thin
//! adapter) is a mechanical translation of its existing
//! `snapshot()`/`restore()` pair (see `jxcl/src/machine.rs`) into this
//! trait — no change to this crate is required.

#![forbid(unsafe_code)]

/// A fully-owned copy of a machine's complete architectural state.
///
/// Mirrors the shape of `jxcl::machine::MachineSnapshot` (general
/// registers, PC/SP/FP, flags, the full memory image, halt/fault state
/// and the cycle counter) but is expressed with plain, machine-agnostic
/// types (`Vec<u64>` rather than a fixed-size array tied to one ISA's
/// register count) so it can be produced by any `SnapshotRestore`
/// implementor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// General-purpose register values, in register-index order.
    pub registers: Vec<u64>,
    pub pc: u64,
    pub sp: u64,
    pub fp: u64,
    pub flags: u64,
    /// The complete memory image at the time of the snapshot.
    pub memory: Vec<u8>,
    pub halted: bool,
    /// A `Debug`-formatted fault name, or `None` if the machine had not
    /// faulted. Kept as text rather than a concrete `ExecutionFault` so
    /// this crate does not need to depend on the ISA/execution crates'
    /// error types.
    pub fault: Option<String>,
    pub cycle_count: u64,
}

impl Snapshot {
    /// A convenience constructor for building a snapshot by hand (used
    /// by tests and by any adapter that wants to construct one field at
    /// a time).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registers: Vec<u64>,
        pc: u64,
        sp: u64,
        fp: u64,
        flags: u64,
        memory: Vec<u8>,
        halted: bool,
        fault: Option<String>,
        cycle_count: u64,
    ) -> Self {
        Snapshot {
            registers,
            pc,
            sp,
            fp,
            flags,
            memory,
            halted,
            fault,
            cycle_count,
        }
    }
}

/// Implemented by any machine-state type that can be captured into a
/// [`Snapshot`] and later reset back to exactly that state.
///
/// The contract every implementor must satisfy (and which
/// [`assert_snapshot_restore_roundtrip`] below mechanically checks):
///
/// ```text
/// let before = machine.snapshot();
/// mutate(&mut machine);
/// machine.restore(before.clone());
/// assert_eq!(machine.snapshot(), before);
/// ```
pub trait SnapshotRestore {
    /// Capture the complete current state as an owned [`Snapshot`].
    fn snapshot(&self) -> Snapshot;

    /// Reset the complete state to exactly what `snapshot` describes,
    /// including anything (e.g. a pending-signal flag) that is not
    /// itself part of `Snapshot` but must be cleared by any fresh
    /// restore.
    fn restore(&mut self, snapshot: Snapshot);
}

/// Snapshot `machine`, apply `mutate`, restore the original snapshot,
/// and assert the machine is byte-for-byte identical to how it started.
///
/// This is the reusable core of the determinism property test: given
/// *any* `SnapshotRestore` implementor and *any* mutation closure, the
/// round trip must be lossless. Property tests for a concrete machine
/// type (once one exists) should call this directly instead of
/// duplicating the assertion.
pub fn assert_snapshot_restore_roundtrip<M: SnapshotRestore>(
    machine: &mut M,
    mutate: impl FnOnce(&mut M),
) {
    let before = machine.snapshot();
    mutate(machine);
    machine.restore(before.clone());
    let after = machine.snapshot();
    assert_eq!(
        after, before,
        "SnapshotRestore round trip lost or changed state"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny deterministic PRNG, matching the style used by
    /// `jxcl/tests/common/mod.rs` (fixed-seed xorshift64, no `rand`
    /// dependency) so the property test below is fully reproducible.
    struct Xorshift64 {
        state: u64,
    }

    impl Xorshift64 {
        fn new(seed: u64) -> Self {
            Xorshift64 {
                state: if seed == 0 {
                    0xDEAD_BEEF_CAFE_BABE
                } else {
                    seed
                },
            }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }
    }

    /// A minimal, real, self-contained stand-in for a steppable
    /// architectural machine: general registers, PC/SP/FP, flags, a
    /// byte-addressable memory image, and halt/fault/cycle-count
    /// bookkeeping — exactly the fields `Snapshot` captures. This is
    /// the "any `SnapshotRestore` implementor" the trait is designed
    /// for; a real `jxcl-machine`/`jxcl` adapter is a drop-in
    /// replacement with the same shape (see `jxcl/src/machine.rs`'s
    /// `MachineState::snapshot`/`restore`).
    #[derive(Debug, Clone)]
    struct DemoMachine {
        registers: Vec<u64>,
        pc: u64,
        sp: u64,
        fp: u64,
        flags: u64,
        memory: Vec<u8>,
        halted: bool,
        fault: Option<String>,
        cycle_count: u64,
    }

    impl DemoMachine {
        fn new(register_count: usize, memory_size: usize) -> Self {
            DemoMachine {
                registers: vec![0; register_count],
                pc: 0,
                sp: memory_size as u64,
                fp: 0,
                flags: 0,
                memory: vec![0; memory_size],
                halted: false,
                fault: None,
                cycle_count: 0,
            }
        }
    }

    impl SnapshotRestore for DemoMachine {
        fn snapshot(&self) -> Snapshot {
            Snapshot::new(
                self.registers.clone(),
                self.pc,
                self.sp,
                self.fp,
                self.flags,
                self.memory.clone(),
                self.halted,
                self.fault.clone(),
                self.cycle_count,
            )
        }

        fn restore(&mut self, snapshot: Snapshot) {
            self.registers = snapshot.registers;
            self.pc = snapshot.pc;
            self.sp = snapshot.sp;
            self.fp = snapshot.fp;
            self.flags = snapshot.flags;
            self.memory = snapshot.memory;
            self.halted = snapshot.halted;
            self.fault = snapshot.fault;
            self.cycle_count = snapshot.cycle_count;
        }
    }

    #[test]
    fn snapshot_carries_every_field() {
        let mut m = DemoMachine::new(4, 16);
        m.registers[1] = 0xABCD;
        m.pc = 4;
        m.flags = 0b0101;
        m.memory[2] = 0x7F;
        m.cycle_count = 9;

        let snap = m.snapshot();
        assert_eq!(snap.registers, vec![0, 0xABCD, 0, 0]);
        assert_eq!(snap.pc, 4);
        assert_eq!(snap.flags, 0b0101);
        assert_eq!(snap.memory[2], 0x7F);
        assert_eq!(snap.cycle_count, 9);
    }

    #[test]
    fn restore_resets_every_field_including_ones_not_touched_since() {
        let mut m = DemoMachine::new(2, 8);
        let clean = m.snapshot();

        m.registers[0] = 42;
        m.pc = 100;
        m.halted = true;
        m.fault = Some("InvalidOpcode".to_string());
        m.cycle_count = 1000;

        m.restore(clean.clone());
        assert_eq!(m.snapshot(), clean);
        assert!(!m.halted);
        assert!(m.fault.is_none());
    }

    #[test]
    fn snapshot_restore_is_lossless_under_one_random_mutation() {
        let mut rng = Xorshift64::new(7);
        let mut m = DemoMachine::new(8, 256);
        for r in m.registers.iter_mut() {
            *r = rng.next_u64();
        }
        m.pc = rng.next_u64() % 256;
        m.flags = rng.next_u64() & 0b1111;
        m.cycle_count = rng.next_u64();

        assert_snapshot_restore_roundtrip(&mut m, |m| {
            for r in m.registers.iter_mut() {
                *r = rng.next_u64();
            }
            m.pc = rng.next_u64() % 256;
            m.memory[0] = (rng.next_u64() & 0xFF) as u8;
            m.halted = true;
            m.cycle_count = rng.next_u64();
        });
    }

    /// The full property test (spec source:
    /// `jxcl/tests/property_tests.rs`'s
    /// `machine_snapshot_restore_is_lossless_under_random_mutation`):
    /// across many independently-seeded random mutations, snapshot then
    /// restore must always exactly undo the mutation.
    #[test]
    fn snapshot_restore_is_lossless_under_random_mutation_property() {
        let mut rng = Xorshift64::new(0x1234_5678);
        const TRIALS: usize = 200;

        for trial in 0..TRIALS {
            let mut m = DemoMachine::new(16, 512);
            for r in m.registers.iter_mut() {
                *r = rng.next_u64();
            }
            m.pc = rng.next_u64() % 512;
            m.sp = rng.next_u64() % 512;
            m.fp = rng.next_u64() % 512;
            m.flags = rng.next_u64() & 0b1111;
            for b in m.memory.iter_mut() {
                *b = (rng.next_u64() & 0xFF) as u8;
            }
            m.cycle_count = rng.next_u64();

            let before = m.snapshot();
            // Mutate every field the snapshot captures, non-trivially.
            for r in m.registers.iter_mut() {
                *r ^= rng.next_u64();
            }
            m.pc = m.pc.wrapping_add(rng.next_u64());
            m.sp = m.sp.wrapping_sub(rng.next_u64());
            m.fp ^= rng.next_u64();
            m.flags ^= rng.next_u64();
            for b in m.memory.iter_mut() {
                *b ^= (rng.next_u64() & 0xFF) as u8;
            }
            m.halted = true;
            m.fault = Some("SomeFault".to_string());
            m.cycle_count = m.cycle_count.wrapping_add(1);

            m.restore(before.clone());
            assert_eq!(
                m.snapshot(),
                before,
                "trial {trial}: snapshot/restore round trip was lossy"
            );
        }
    }

    #[test]
    fn snapshot_equality_is_structural_not_by_reference() {
        let m1 = DemoMachine::new(2, 4);
        let m2 = DemoMachine::new(2, 4);
        assert_eq!(m1.snapshot(), m2.snapshot());
    }
}
