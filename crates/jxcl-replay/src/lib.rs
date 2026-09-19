//! Deterministically replays a recorded [`jxcl_trace::TraceLog`] to
//! reconstruct machine state at any step, without re-running the
//! original inputs.
//!
//! [`Replay::seek_to_step`] starts from a known initial
//! [`jxcl_determinism::Snapshot`] and applies each recorded step's
//! register/flags/stack-pointer/memory deltas in order, producing the
//! exact [`Snapshot`] the live machine had after executing that many
//! steps — no re-execution of the original program is needed, which is
//! what makes this useful for a debugger's "jump to step N" or a
//! profiler's "what did register R1 look like right before the crash"
//! query.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` additionally lists `jxcl-machine` as a dependency
//! (still a scaffolded placeholder as of this batch). This crate does
//! not need `jxcl-machine`'s concrete type: it operates purely on
//! [`jxcl_trace::TraceLog`] (already real, from this same batch) and
//! [`jxcl_determinism::Snapshot`] (also real, from this same batch),
//! which is exactly the state `Replay` reconstructs. Once
//! `jxcl-machine::Machine` implements `SnapshotRestore`, a caller can
//! `replay.seek_to_step(...)` and feed the resulting `Snapshot`
//! straight into `Machine::restore`.

#![forbid(unsafe_code)]

use jxcl_determinism::Snapshot;
use jxcl_trace::TraceLog;

/// Errors [`Replay::seek_to_step`] can return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// `n` is not a valid index into the trace log (it has `len` steps,
    /// valid indices are `0..len`).
    StepOutOfRange { requested: usize, len: usize },
    /// A recorded register-change index did not fit inside the initial
    /// snapshot's register file.
    RegisterIndexOutOfRange { index: u8, register_count: usize },
    /// A recorded memory write fell outside the initial snapshot's
    /// memory image.
    MemoryWriteOutOfRange { address: u64, len: usize },
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::StepOutOfRange { requested, len } => write!(
                f,
                "requested step {requested} but the trace only has {len} step(s)"
            ),
            ReplayError::RegisterIndexOutOfRange {
                index,
                register_count,
            } => write!(
                f,
                "trace referenced register {index} but the snapshot only has {register_count} register(s)"
            ),
            ReplayError::MemoryWriteOutOfRange { address, len } => write!(
                f,
                "trace referenced a write of {len} byte(s) at address {address:#x} outside the snapshot's memory"
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Replays a [`TraceLog`] against an initial [`Snapshot`] to
/// deterministically reconstruct machine state at any recorded step.
pub struct Replay<'a> {
    trace: &'a TraceLog,
    initial: Snapshot,
}

impl<'a> Replay<'a> {
    /// Begin a replay session over `trace`, starting from `initial` —
    /// the snapshot taken *before* the first step in `trace` was
    /// executed.
    pub fn new(trace: &'a TraceLog, initial: Snapshot) -> Self {
        Replay { trace, initial }
    }

    /// Reconstruct machine state as of immediately after step `n`
    /// (0-indexed) was executed, by replaying steps `0..=n`'s deltas
    /// onto the initial snapshot.
    pub fn seek_to_step(&self, n: usize) -> Result<Snapshot, ReplayError> {
        if n >= self.trace.len() {
            return Err(ReplayError::StepOutOfRange {
                requested: n,
                len: self.trace.len(),
            });
        }

        let mut state = self.initial.clone();
        for step in self.trace.iter().take(n + 1) {
            apply_step(&mut state, step)?;
        }
        Ok(state)
    }
}

fn apply_step(state: &mut Snapshot, step: &jxcl_trace::TraceStep) -> Result<(), ReplayError> {
    for (index, _old, new) in &step.register_changes {
        let register_count = state.registers.len();
        let slot = state.registers.get_mut(*index as usize).ok_or(
            ReplayError::RegisterIndexOutOfRange {
                index: *index,
                register_count,
            },
        )?;
        *slot = *new;
    }

    state.flags = step.flags_after;
    state.sp = step.sp_after;
    state.pc = step.pc_after;

    for (addr, _old, new) in &step.memory_writes {
        let start = *addr as usize;
        let end = start
            .checked_add(new.len())
            .ok_or(ReplayError::MemoryWriteOutOfRange {
                address: *addr,
                len: new.len(),
            })?;
        let dest = state
            .memory
            .get_mut(start..end)
            .ok_or(ReplayError::MemoryWriteOutOfRange {
                address: *addr,
                len: new.len(),
            })?;
        dest.copy_from_slice(new);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_trace::TraceStep;

    /// A minimal, real "live machine" used only to produce the ground
    /// truth this test checks replay against: it applies each step's
    /// intent directly (rather than via `apply_step`, which is the code
    /// under test) so the comparison is meaningful.
    #[derive(Clone, Debug, PartialEq)]
    struct LiveState {
        registers: Vec<u64>,
        pc: u64,
        sp: u64,
        flags: u64,
        memory: Vec<u8>,
    }

    fn to_snapshot(live: &LiveState) -> Snapshot {
        Snapshot::new(
            live.registers.clone(),
            live.pc,
            live.sp,
            0,
            live.flags,
            live.memory.clone(),
            false,
            None,
            0,
        )
    }

    /// Records a short hand-scripted "run" (3 instructions: two
    /// register writes and a memory store) as both a live final state
    /// and the equivalent `TraceLog`, mirroring how
    /// `jxcl::debugger::trace_step` would record a real run (spec §30).
    fn record_short_run() -> (Snapshot, TraceLog, Snapshot) {
        let mut live = LiveState {
            registers: vec![0, 0, 0],
            pc: 0,
            sp: 256,
            flags: 0,
            memory: vec![0; 128],
        };
        let initial = to_snapshot(&live);
        let mut trace = TraceLog::new();

        // Step 0: MOVI R1, 42
        let before = live.clone();
        live.registers[1] = 42;
        live.pc = 4;
        trace.push(TraceStep {
            pc: before.pc,
            pc_after: live.pc,
            instruction: "MOVI R1, 42".to_string(),
            register_changes: vec![(1, before.registers[1], live.registers[1])],
            flags_before: before.flags,
            flags_after: live.flags,
            sp_before: before.sp,
            sp_after: live.sp,
            memory_writes: vec![],
        });

        // Step 1: MOVI R2, 100
        let before = live.clone();
        live.registers[2] = 100;
        live.pc = 8;
        trace.push(TraceStep {
            pc: before.pc,
            pc_after: live.pc,
            instruction: "MOVI R2, 100".to_string(),
            register_changes: vec![(2, before.registers[2], live.registers[2])],
            flags_before: before.flags,
            flags_after: live.flags,
            sp_before: before.sp,
            sp_after: live.sp,
            memory_writes: vec![],
        });

        // Step 2: STORE [64], R1  (store the low byte of R1 at address 64)
        let before = live.clone();
        let old_byte = live.memory[64];
        live.memory[64] = live.registers[1] as u8;
        live.pc = 12;
        trace.push(TraceStep {
            pc: before.pc,
            pc_after: live.pc,
            instruction: "STORE [64], R1".to_string(),
            register_changes: vec![],
            flags_before: before.flags,
            flags_after: live.flags,
            sp_before: before.sp,
            sp_after: live.sp,
            memory_writes: vec![(64, vec![old_byte], vec![live.memory[64]])],
        });

        let final_live = to_snapshot(&live);
        (initial, trace, final_live)
    }

    #[test]
    fn seek_to_last_step_reconstructs_the_live_final_state() {
        let (initial, trace, final_live) = record_short_run();
        let replay = Replay::new(&trace, initial);

        let reconstructed = replay
            .seek_to_step(trace.len() - 1)
            .expect("last step must be reachable");

        assert_eq!(reconstructed, final_live);
    }

    #[test]
    fn seek_to_intermediate_step_reflects_only_deltas_so_far() {
        let (initial, trace, _final_live) = record_short_run();
        let replay = Replay::new(&trace, initial);

        let after_first = replay.seek_to_step(0).unwrap();
        assert_eq!(after_first.registers[1], 42);
        assert_eq!(after_first.registers[2], 0, "step 1 hasn't run yet");
        assert_eq!(after_first.memory[64], 0, "step 2 hasn't run yet");

        let after_second = replay.seek_to_step(1).unwrap();
        assert_eq!(after_second.registers[2], 100);
        assert_eq!(after_second.memory[64], 0, "step 2 still hasn't run");
    }

    #[test]
    fn seeking_past_the_end_is_a_clean_error_not_a_panic() {
        let (initial, trace, _) = record_short_run();
        let replay = Replay::new(&trace, initial);
        let err = replay.seek_to_step(trace.len()).unwrap_err();
        assert_eq!(
            err,
            ReplayError::StepOutOfRange {
                requested: trace.len(),
                len: trace.len()
            }
        );
    }

    #[test]
    fn replay_is_deterministic_across_repeated_seeks() {
        let (initial, trace, _) = record_short_run();
        let replay = Replay::new(&trace, initial);
        let a = replay.seek_to_step(2).unwrap();
        let b = replay.seek_to_step(2).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn out_of_range_register_index_is_a_clean_error() {
        let initial = Snapshot::new(vec![0, 0], 0, 0, 0, 0, vec![0; 8], false, None, 0);
        let mut trace = TraceLog::new();
        trace.push(TraceStep {
            pc: 0,
            pc_after: 4,
            instruction: "BAD".to_string(),
            register_changes: vec![(5, 0, 1)], // only 2 registers exist
            flags_before: 0,
            flags_after: 0,
            sp_before: 0,
            sp_after: 0,
            memory_writes: vec![],
        });
        let replay = Replay::new(&trace, initial);
        assert!(matches!(
            replay.seek_to_step(0),
            Err(ReplayError::RegisterIndexOutOfRange { index: 5, .. })
        ));
    }

    #[test]
    fn out_of_range_memory_write_is_a_clean_error() {
        let initial = Snapshot::new(vec![0], 0, 0, 0, 0, vec![0; 4], false, None, 0);
        let mut trace = TraceLog::new();
        trace.push(TraceStep {
            pc: 0,
            pc_after: 4,
            instruction: "BAD".to_string(),
            register_changes: vec![],
            flags_before: 0,
            flags_after: 0,
            sp_before: 0,
            sp_after: 0,
            memory_writes: vec![(100, vec![0], vec![1])], // outside 4-byte memory
        });
        let replay = Replay::new(&trace, initial);
        assert!(matches!(
            replay.seek_to_step(0),
            Err(ReplayError::MemoryWriteOutOfRange { address: 100, .. })
        ));
    }
}
