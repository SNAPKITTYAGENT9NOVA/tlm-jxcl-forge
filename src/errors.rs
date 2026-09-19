//! Structured error model for the JXCL forge (spec §42).
//!
//! Every fallible operation in the forge returns one of these concrete
//! error types instead of relying on host-language panics/exceptions as
//! the architectural definition of failure (spec §11, §42).

use std::fmt;

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
        write!(f, "assembler error at line {}: {:?}: {}", self.line, self.kind, self.reason)
    }
}
impl std::error::Error for AssemblerError {}

/// Errors produced while decoding a byte stream into a structured instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub kind: DecodeErrorKind,
    /// Byte offset (relative to the start of the code section) where decoding failed.
    pub offset: u64,
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
        write!(f, "decode error at offset {:#x}: {:?}: {}", self.offset, self.kind, self.reason)
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
    pub offset: u64,
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
        write!(f, "validation error at offset {:#x}: {:?}: {}", self.offset, self.kind, self.reason)
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
