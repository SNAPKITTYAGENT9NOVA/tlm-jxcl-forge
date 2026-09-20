// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The machine's exception model: illegal opcode, misaligned access,
//! division by zero, and the trap-handling hook.
//!
//! `Exception` generalizes `crates/jxcl/src/errors.rs`'s `ExecutionFault`
//! (spec §11) into the vocabulary the new execution-group crates share:
//! everything the fetch/decode/execute loop, the page table, and the
//! stack/heap allocators can signal as an architectural fault, plus a
//! `TrapHandler` contract so a host (debugger, simulator, RPC server) can
//! plug in its own response to a fault or a SYS/TRAP signal instead of the
//! machine unilaterally deciding what happens next.
#![forbid(unsafe_code)]

use std::fmt;

use jxcl_errors::MemoryFault;
use jxcl_types::Address;

/// An architectural exception: every non-local, non-recoverable-by-the-
/// current-instruction condition the machine can signal. Values, not host
/// panics (spec §11, §42): every fallible operation in the execution group
/// returns one of these instead of unwinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    /// The fetched opcode byte has no registered instruction definition.
    IllegalOpcode,
    /// A decoded register field named a register outside the architectural
    /// register file.
    InvalidRegister,
    /// An operand's shape or encoded value was structurally illegal
    /// (e.g. a format the decoder produced but execution can't use).
    IllegalOperand,
    /// A load/store address was not a multiple of its access width.
    MisalignedAccess { address: Address },
    /// Integer division or remainder by zero.
    DivisionByZero,
    /// A PUSH (or CALL's implicit push) ran out of stack space.
    StackOverflow,
    /// A POP (or RET's implicit pop) was attempted on an empty stack.
    StackUnderflow,
    /// A virtual address had no mapping in the page table.
    PageFault { address: Address },
    /// An access was denied by the address space's permission model.
    PermissionViolation { address: Address },
    /// Any other memory-subsystem fault (bounds, alignment) that doesn't
    /// need its own dedicated variant here; wraps `jxcl-errors::MemoryFault`.
    Memory(MemoryFault),
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exception::IllegalOpcode => write!(f, "illegal opcode"),
            Exception::InvalidRegister => write!(f, "invalid register"),
            Exception::IllegalOperand => write!(f, "illegal operand"),
            Exception::MisalignedAccess { address } => {
                write!(f, "misaligned access at {address}")
            }
            Exception::DivisionByZero => write!(f, "division by zero"),
            Exception::StackOverflow => write!(f, "stack overflow"),
            Exception::StackUnderflow => write!(f, "stack underflow"),
            Exception::PageFault { address } => write!(f, "page fault at {address}"),
            Exception::PermissionViolation { address } => {
                write!(f, "permission violation at {address}")
            }
            Exception::Memory(m) => write!(f, "memory fault: {m}"),
        }
    }
}
impl std::error::Error for Exception {}

impl From<MemoryFault> for Exception {
    fn from(m: MemoryFault) -> Self {
        Exception::Memory(m)
    }
}

/// What a `TrapHandler` decides should happen after it has observed an
/// `Exception` (or a signal delivered as a `TrapHandler::handle` call).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapAction {
    /// Stop the machine; no further instructions execute.
    Halt,
    /// The handler dealt with the condition; execution continues normally.
    Resume,
}

/// A host-pluggable response to a machine exception (spec §26's SYS/TRAP
/// "purely data for whatever host integration chooses to interpret it",
/// generalized to cover faults too). The base execution engine calls into
/// a `TrapHandler` rather than baking in one fixed policy, so a debugger
/// can choose to pause instead of halting, a fuzzer can choose to record
/// and continue, and so on.
pub trait TrapHandler {
    /// Called with every `Exception` the machine raises. The return value
    /// tells the caller whether to keep running.
    fn handle(&mut self, exception: Exception) -> TrapAction;
}

/// The simplest possible `TrapHandler`: every exception halts the
/// machine. This matches `crates/jxcl/src/execution.rs`'s unconditional
/// `fault()` behavior and is a reasonable default for callers that don't
/// need custom trap handling.
#[derive(Debug, Clone, Copy, Default)]
pub struct HaltOnException;

impl TrapHandler for HaltOnException {
    fn handle(&mut self, _exception: Exception) -> TrapAction {
        TrapAction::Halt
    }
}

/// A `TrapHandler` that records every exception it observes and always
/// resumes -- useful for a fuzzer or a test harness that wants to keep
/// running while collecting what went wrong.
#[derive(Debug, Clone, Default)]
pub struct RecordingTrapHandler {
    pub observed: Vec<Exception>,
}

impl TrapHandler for RecordingTrapHandler {
    fn handle(&mut self, exception: Exception) -> TrapAction {
        self.observed.push(exception);
        TrapAction::Resume
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_fault_converts_into_exception() {
        let e: Exception = MemoryFault::AlignmentFault.into();
        assert_eq!(e, Exception::Memory(MemoryFault::AlignmentFault));
    }

    #[test]
    fn display_includes_context_fields() {
        let e = Exception::PageFault {
            address: Address::new(0x1000),
        };
        let text = e.to_string();
        assert!(text.contains("page fault"));
        assert!(text.contains("0x0000000000001000"));
    }

    #[test]
    fn halt_on_exception_always_halts() {
        let mut h = HaltOnException;
        assert_eq!(h.handle(Exception::DivisionByZero), TrapAction::Halt);
        assert_eq!(h.handle(Exception::IllegalOpcode), TrapAction::Halt);
    }

    #[test]
    fn recording_handler_resumes_and_records() {
        let mut h = RecordingTrapHandler::default();
        assert_eq!(h.handle(Exception::StackOverflow), TrapAction::Resume);
        assert_eq!(h.handle(Exception::StackUnderflow), TrapAction::Resume);
        assert_eq!(
            h.observed,
            vec![Exception::StackOverflow, Exception::StackUnderflow]
        );
    }

    #[test]
    fn exception_is_usable_as_trait_object_error() {
        let boxed: Box<dyn std::error::Error> = Box::new(Exception::DivisionByZero);
        assert_eq!(boxed.to_string(), "division by zero");
    }
}
