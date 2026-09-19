//! TLM JXCL — a deterministic, raw hardware-oriented dense instruction-set
//! architecture forge (see docs/ISA_SPEC.md for the full architectural
//! specification).

pub mod alu;
pub mod assembler;
pub mod binary;
pub mod control;
pub mod debugger;
pub mod disassembler;
pub mod encoding;
pub mod errors;
pub mod execution;
pub mod isa;
pub mod machine;
pub mod memory;
pub mod validator;
