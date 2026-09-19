//! Interactive/trace-mode debugger: step, breakpoint, and inspect a
//! running machine.
//!
//! Owns: The [`Debugger`] driver.
//!
//! Ported from `crates/jxcl/src/debugger.rs`'s `trace_step` (spec §30):
//! that function fetches, decodes, renders, executes and diffs one
//! instruction against a `jxcl::machine::MachineState`, returning a
//! `TraceEntry`. This crate reuses [`jxcl-trace`]'s `TraceStep` as that
//! record type (per this batch's instructions: "build on jxcl-machine +
//! jxcl-trace for its trace-mode output, not a separate copy") instead
//! of redefining `TraceEntry` here, and adds the breakpoint/run-loop
//! driver logic around it that `jxcl::debugger` left to its CLI caller.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` lists `jxcl-machine` and `jxcl-execution` as this
//! crate's workspace dependencies (`Cargo.toml` declares both as path
//! dependencies to match); both are still scaffolded placeholders as of
//! this batch, with no `MachineState`/`step` API yet to drive. Rather
//! than block on that, [`Debugger`] is generic over a small [`Steppable`]
//! trait defined locally, capturing exactly the shape
//! `jxcl::debugger::trace_step` already needs from a machine: execute one
//! instruction and report the resulting [`TraceStep`]. Once
//! `jxcl-machine`/`jxcl-execution` land, a wrapper type around their real
//! `MachineState` that implements `Steppable` by calling
//! `jxcl_execution::step` (and rendering the executed instruction's text,
//! as `jxcl::debugger::trace_step` already does) plugs into `Debugger`
//! without any change to this crate's public API.
#![forbid(unsafe_code)]

use jxcl_trace::{TraceLog, TraceStep};
use std::collections::BTreeSet;

/// Anything that can execute one instruction and report what
/// architecturally changed, in the shape [`jxcl-trace`]'s [`TraceStep`]
/// already captures. This is the seam [`Debugger`] steps through; see
/// this module's "Cross-batch integration note" for what implements it
/// once `jxcl-machine`/`jxcl-execution` land.
pub trait Steppable {
    /// Execute exactly one instruction and return a full trace of what
    /// changed. Implementations should make this idempotent once halted
    /// (matching `jxcl::execution::step`'s "further calls are no-ops"
    /// contract), returning a no-op [`TraceStep`] rather than panicking.
    fn step(&mut self) -> TraceStep;

    /// True once the machine has halted or faulted; further [`Self::step`]
    /// calls are expected to be no-ops.
    fn is_halted(&self) -> bool;

    /// The program counter the next [`Self::step`] call will fetch from.
    fn pc(&self) -> u64;
}

/// Outcome of [`Debugger::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// Stopped because the PC after some step matched a breakpoint.
    HitBreakpoint(u64),
    /// The machine halted (or faulted -- `Steppable` reports both via
    /// `is_halted`) during this run.
    Halted,
    /// Neither happened within the requested step budget.
    StepLimitReached,
}

/// The interactive/trace-mode debugger driver (spec §30): single-steps a
/// [`Steppable`] machine, manages breakpoints by PC, and records every
/// executed step's [`TraceStep`] into a [`TraceLog`] history.
pub struct Debugger<M: Steppable> {
    machine: M,
    breakpoints: BTreeSet<u64>,
    history: TraceLog,
}

impl<M: Steppable> Debugger<M> {
    /// Wrap `machine`, starting with no breakpoints and an empty history.
    pub fn new(machine: M) -> Self {
        Debugger {
            machine,
            breakpoints: BTreeSet::new(),
            history: TraceLog::new(),
        }
    }

    /// Add a breakpoint at `pc`. Returns `false` if one was already set
    /// there.
    pub fn add_breakpoint(&mut self, pc: u64) -> bool {
        self.breakpoints.insert(pc)
    }

    /// Remove a breakpoint at `pc`. Returns `false` if none was set there.
    pub fn remove_breakpoint(&mut self, pc: u64) -> bool {
        self.breakpoints.remove(&pc)
    }

    pub fn has_breakpoint(&self, pc: u64) -> bool {
        self.breakpoints.contains(&pc)
    }

    /// Every currently set breakpoint address, in ascending order.
    pub fn breakpoints(&self) -> impl Iterator<Item = &u64> {
        self.breakpoints.iter()
    }

    pub fn is_halted(&self) -> bool {
        self.machine.is_halted()
    }

    pub fn pc(&self) -> u64 {
        self.machine.pc()
    }

    pub fn machine(&self) -> &M {
        &self.machine
    }

    pub fn machine_mut(&mut self) -> &mut M {
        &mut self.machine
    }

    /// The full step history recorded so far.
    pub fn history(&self) -> &TraceLog {
        &self.history
    }

    /// Single-step the machine, recording the resulting trace into
    /// [`Self::history`] and returning a reference to it. Returns `None`
    /// without stepping (and without recording anything) if the machine
    /// was already halted.
    pub fn step(&mut self) -> Option<&TraceStep> {
        if self.machine.is_halted() {
            return None;
        }
        let step = self.machine.step();
        self.history.push(step);
        self.history.get(self.history.len() - 1)
    }

    /// Single-step repeatedly until the machine halts, a breakpoint's PC
    /// is reached, or `max_steps` steps have executed -- whichever comes
    /// first. Always executes at least one step before checking for a
    /// breakpoint, so calling `run` while already stopped at one makes
    /// progress instead of returning immediately.
    pub fn run(&mut self, max_steps: usize) -> RunOutcome {
        for _ in 0..max_steps {
            if self.machine.is_halted() {
                return RunOutcome::Halted;
            }
            self.step();
            if self.machine.is_halted() {
                return RunOutcome::Halted;
            }
            if self.breakpoints.contains(&self.machine.pc()) {
                return RunOutcome::HitBreakpoint(self.machine.pc());
            }
        }
        RunOutcome::StepLimitReached
    }
}

/// Render one [`TraceStep`] the way `jxcl::debugger::TraceEntry`'s
/// `Display` impl does (spec §30): the PC and instruction text, then any
/// register/SP/flags/memory deltas, one per line.
pub fn format_trace_step(step: &TraceStep) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "{:08X}  {}", step.pc, step.instruction);
    for (id, old, new) in &step.register_changes {
        let _ = writeln!(out, "  R{id}: {old:#x} -> {new:#x}");
    }
    if step.sp_before != step.sp_after {
        let _ = writeln!(out, "  SP: {:#x} -> {:#x}", step.sp_before, step.sp_after);
    }
    if step.flags_before != step.flags_after {
        let _ = writeln!(
            out,
            "  FLAGS: {:#06b} -> {:#06b}",
            step.flags_before, step.flags_after
        );
    }
    for (addr, old, new) in &step.memory_writes {
        let _ = writeln!(
            out,
            "  MEM[{:#x}..{:#x}]: {:02x?} -> {:02x?}",
            addr,
            addr + old.len() as u64,
            old,
            new
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `Steppable` for testing `Debugger`'s driver logic in
    /// isolation from any concrete machine: it increments R1 by one and
    /// advances PC by 4 each step, halting once R1 reaches `limit`.
    struct Counter {
        pc: u64,
        value: u64,
        limit: u64,
        halted: bool,
    }

    impl Counter {
        fn new(limit: u64) -> Self {
            Counter {
                pc: 0,
                value: 0,
                limit,
                halted: false,
            }
        }
    }

    impl Steppable for Counter {
        fn step(&mut self) -> TraceStep {
            if self.halted {
                return TraceStep::unchanged(self.pc, self.pc, "HALT", 0, 0);
            }
            let pc = self.pc;
            let before = self.value;
            self.value += 1;
            self.pc += 4;
            if self.value >= self.limit {
                self.halted = true;
            }
            TraceStep {
                pc,
                pc_after: self.pc,
                instruction: format!("INC R1 ; -> {}", self.value),
                register_changes: vec![(1, before, self.value)],
                flags_before: 0,
                flags_after: 0,
                sp_before: 0,
                sp_after: 0,
                memory_writes: Vec::new(),
            }
        }

        fn is_halted(&self) -> bool {
            self.halted
        }

        fn pc(&self) -> u64 {
            self.pc
        }
    }

    #[test]
    fn step_records_history_and_returns_the_step() {
        let mut dbg = Debugger::new(Counter::new(5));
        let step = dbg.step().unwrap().clone();
        assert_eq!(step.register_changes, vec![(1, 0, 1)]);
        assert_eq!(dbg.history().len(), 1);
        assert_eq!(dbg.pc(), 4);
    }

    #[test]
    fn step_returns_none_once_halted() {
        let mut dbg = Debugger::new(Counter::new(1));
        assert_eq!(dbg.run(10), RunOutcome::Halted);
        assert!(dbg.is_halted());
        assert!(dbg.step().is_none());
        // No step was recorded for the no-op call above.
        assert_eq!(dbg.history().len(), 1);
    }

    #[test]
    fn run_halts_when_the_machine_halts() {
        let mut dbg = Debugger::new(Counter::new(3));
        let outcome = dbg.run(100);
        assert_eq!(outcome, RunOutcome::Halted);
        assert_eq!(dbg.history().len(), 3);
        assert_eq!(dbg.machine().value, 3);
    }

    #[test]
    fn run_stops_at_a_breakpoint() {
        let mut dbg = Debugger::new(Counter::new(100));
        dbg.add_breakpoint(8); // PC after the 2nd step
        let outcome = dbg.run(100);
        assert_eq!(outcome, RunOutcome::HitBreakpoint(8));
        assert_eq!(dbg.history().len(), 2);
        assert!(!dbg.is_halted());
    }

    #[test]
    fn run_respects_the_step_limit() {
        let mut dbg = Debugger::new(Counter::new(1000));
        let outcome = dbg.run(3);
        assert_eq!(outcome, RunOutcome::StepLimitReached);
        assert_eq!(dbg.history().len(), 3);
    }

    #[test]
    fn breakpoint_management_round_trips() {
        let mut dbg = Debugger::new(Counter::new(10));
        assert!(dbg.add_breakpoint(4));
        assert!(!dbg.add_breakpoint(4)); // already set
        assert!(dbg.has_breakpoint(4));
        assert_eq!(dbg.breakpoints().collect::<Vec<_>>(), vec![&4]);
        assert!(dbg.remove_breakpoint(4));
        assert!(!dbg.has_breakpoint(4));
        assert!(!dbg.remove_breakpoint(4));
    }

    #[test]
    fn format_trace_step_renders_register_and_pc() {
        let mut dbg = Debugger::new(Counter::new(1));
        let step = dbg.step().unwrap().clone();
        let text = format_trace_step(&step);
        assert!(text.starts_with("00000000  "));
        assert!(text.contains("R1: 0x0 -> 0x1"));
    }

    #[test]
    fn format_trace_step_omits_unchanged_sections() {
        let step = TraceStep::unchanged(0, 4, "NOP", 0, 256);
        let text = format_trace_step(&step);
        assert!(!text.contains("SP:"));
        assert!(!text.contains("FLAGS:"));
        assert!(!text.contains("MEM["));
    }
}
