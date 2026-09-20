// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Structured error model for the JXCL forge (spec §42), extracted from
//! the pre-expansion `jxcl::errors` module.
//!
//! Every fallible operation in the forge returns one of these concrete
//! error types instead of relying on host-language panics/exceptions as
//! the architectural definition of failure (spec §11, §42).
//!
//! ## Deviation from `docs/crates.toml`
//!
//! The registry describes this crate's `owns`/`public_api` as a single
//! `Error` enum. The real pre-expansion source (`jxcl/src/errors.rs`)
//! does not have one: it has seven distinct, purpose-built error/fault
//! types (`AssemblerError`, `DecodeError`, `EncodeError`,
//! `ValidationError`, `MemoryFault`, `ExecutionFault`,
//! `BinaryFormatError`), each carrying a `*Kind` enum plus contextual
//! fields (offset, line, reason). Collapsing these into one enum would
//! either lose that context or force every caller to match through an
//! extra layer of indirection for no benefit -- and every downstream
//! extraction crate (`jxcl-encoding`, `jxcl-decoding`, ...) is written
//! against these concrete names already. So this crate keeps the real
//! types verbatim (adjusted only to use `jxcl-types::Address` for
//! byte-offset fields, since that's exactly what those offsets are), and
//! *additionally* provides a genuinely new `Error` enum -- matching the
//! registry's literal `public_api` entry -- for the foundation-level
//! "new" crates in this same batch (`jxcl-bytes`, `jxcl-config`,
//! `jxcl-instruction-validation`) that need one generic, growable error
//! type rather than a bespoke one each.

use std::fmt;

use jxcl_types::Address;

/// Errors produced while assembling JXCL source text into a binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerError {
    pub kind: AssemblerErrorKind,
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblerErrorKind {
    UnknownMnemonic,
    UnknownRegister,
    UnknownLabel,
    DuplicateLabel,
    MalformedImmediate,
    MalformedOperand,
    OperandCountMismatch,
    UnterminatedString,
    UnexpectedToken,
    ImmediateOutOfRange,
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "assembler error at line {}: {:?}: {}",
            self.line, self.kind, self.reason
        )
    }
}
impl std::error::Error for AssemblerError {}

/// Errors produced while decoding a byte stream into a structured instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub kind: DecodeErrorKind,
    /// Offset (relative to the start of the code section) where decoding failed.
    pub offset: Address,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeErrorKind {
    InvalidOpcode,
    TruncatedInstruction,
    InvalidRegister,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "decode error at offset {}: {:?}: {}",
            self.offset, self.kind, self.reason
        )
    }
}
impl std::error::Error for DecodeError {}

/// Errors produced while encoding a structured instruction into bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeError {
    pub kind: EncodeErrorKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeErrorKind {
    InvalidRegister,
    InvalidImmediate,
    InvalidOperandCount,
    InvalidOperandType,
    InvalidDisplacement,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "encode error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for EncodeError {}

/// Errors produced by the static validator (spec §29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    pub offset: Address,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErrorKind {
    BadHeader,
    BadOpcode,
    BadInstructionBoundary,
    BadRegisterOperand,
    BadImmediate,
    BadBranchTarget,
    BadMemoryOperand,
    BadEntryPoint,
    BadSectionBounds,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "validation error at offset {}: {:?}: {}",
            self.offset, self.kind, self.reason
        )
    }
}
impl std::error::Error for ValidationError {}

/// Architectural memory faults (spec §11). These are data, not host exceptions:
/// the execution engine returns them as ordinary values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFault {
    InvalidAddress,
    AlignmentFault,
    ReadViolation,
    WriteViolation,
    ExecuteViolation,
}

impl fmt::Display for MemoryFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for MemoryFault {}

/// Architectural execution faults (spec §11), returned by the execution engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionFault {
    InvalidOpcode,
    InvalidRegister,
    DivideByZero,
    StackFault,
    IllegalOperand,
    Memory(MemoryFault),
}

impl fmt::Display for ExecutionFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for ExecutionFault {}

impl From<MemoryFault> for ExecutionFault {
    fn from(m: MemoryFault) -> Self {
        ExecutionFault::Memory(m)
    }
}

/// Errors produced while parsing/validating the JXCL binary executable header (spec §27).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryFormatError {
    pub kind: BinaryFormatErrorKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryFormatErrorKind {
    TooShort,
    BadMagic,
    UnsupportedVersion,
    UnsupportedArchitecture,
    OffsetOutOfRange,
    SizeOutOfRange,
}

impl fmt::Display for BinaryFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "binary format error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for BinaryFormatError {}

/// A generic error for foundation-level utilities that don't have (or
/// don't warrant) a dedicated architectural error type of their own.
/// See the module-level "Deviation" note for why this exists alongside
/// the concrete types above rather than replacing them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A bounded byte buffer was read from or written to past its end.
    OutOfBounds {
        /// Bytes the operation needed.
        needed: usize,
        /// Bytes actually available from the current position.
        available: usize,
        /// Position within the buffer where the operation was attempted.
        at: u64,
    },
    /// An environment-derived configuration value was present but
    /// malformed (wrong length, invalid encoding, out of range).
    InvalidConfig { name: &'static str, reason: String },
    /// An `Instruction` failed a per-instruction legality check (operand
    /// shape didn't match its opcode, a register operand was out of
    /// range, ...).
    InvalidInstruction { reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutOfBounds {
                needed,
                available,
                at,
            } => write!(
                f,
                "out of bounds at offset {:#x}: needed {} byte(s), {} available",
                at, needed, available
            ),
            Error::InvalidConfig { name, reason } => {
                write!(f, "invalid configuration for {}: {}", name, reason)
            }
            Error::InvalidInstruction { reason } => write!(f, "invalid instruction: {}", reason),
        }
    }
}
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembler_error_display() {
        let e = AssemblerError {
            kind: AssemblerErrorKind::UnknownMnemonic,
            line: 3,
            reason: "FROB".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "assembler error at line 3: UnknownMnemonic: FROB"
        );
    }

    #[test]
    fn decode_error_display_uses_address_hex_formatting() {
        let e = DecodeError {
            kind: DecodeErrorKind::InvalidOpcode,
            offset: Address::new(0xff),
            reason: "no such opcode".to_string(),
        };
        assert!(e.to_string().contains("0x00000000000000ff"));
        assert!(e.to_string().contains("InvalidOpcode"));
    }

    #[test]
    fn memory_fault_converts_into_execution_fault() {
        let fault: ExecutionFault = MemoryFault::AlignmentFault.into();
        assert_eq!(fault, ExecutionFault::Memory(MemoryFault::AlignmentFault));
    }

    #[test]
    fn binary_format_error_display() {
        let e = BinaryFormatError {
            kind: BinaryFormatErrorKind::BadMagic,
            reason: "expected JXCL".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "binary format error: BadMagic: expected JXCL"
        );
    }

    #[test]
    fn generic_error_out_of_bounds_display() {
        let e = Error::OutOfBounds {
            needed: 4,
            available: 1,
            at: 10,
        };
        assert_eq!(
            e.to_string(),
            "out of bounds at offset 0xa: needed 4 byte(s), 1 available"
        );
    }

    #[test]
    fn generic_error_invalid_config_display() {
        let e = Error::InvalidConfig {
            name: "PQ_KEM_SEED",
            reason: "wrong length".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "invalid configuration for PQ_KEM_SEED: wrong length"
        );
    }

    #[test]
    fn errors_are_usable_as_trait_objects() {
        let boxed: Box<dyn std::error::Error> = Box::new(Error::InvalidInstruction {
            reason: "bad shape".to_string(),
        });
        assert!(boxed.to_string().contains("bad shape"));
    }
}
