// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! ISA-wide constants (spec §3, §10, §17, §27, §40).
//!
//! These values are the frozen architectural parameters of TLM JXCL.
//! Nothing outside this file may silently redefine them (spec §3: "Do not
//! silently change widths").

/// Width, in bits, of every general-purpose register and of PC/SP/FP.
pub const REGISTER_WIDTH_BITS: u32 = 64;

/// Number of architectural general-purpose registers (R0..=R31).
pub const NUM_GP_REGISTERS: usize = 32;

/// R0 is hardwired to zero (architectural decision, spec §5):
/// reads always yield 0, writes are discarded. This mirrors common
/// RISC practice and gives the ISA a cheap "always zero" operand
/// without a dedicated addressing mode.
pub const R0_HARDWIRED_ZERO: bool = true;

/// Address space width. With 64-bit registers the full 64-bit address
/// space is architecturally addressable; the reference machine backs
/// it with a bounded flat buffer (spec §10).
pub const ADDRESS_WIDTH_BITS: u32 = 64;

/// JXCL is little-endian for all multi-byte memory and immediate encodings
/// (spec §10: "Default: little endian unless the specification explicitly
/// selects another format." No other format is selected here.)
pub const LITTLE_ENDIAN: bool = true;

/// Stack grows downward: PUSH decrements SP before writing, POP reads
/// then increments SP (spec §15).
pub const STACK_GROWS_DOWNWARD: bool = true;

/// PC convention (spec §14): PC holds the address of the instruction
/// currently being fetched. Branch/CALL displacements are relative to
/// the address of the *next sequential instruction*, i.e.
/// `pc_after_fetch = pc_of_instruction + instruction_length`,
/// `target = pc_after_fetch + displacement`.
pub const PC_POINTS_TO_CURRENT_INSTRUCTION: bool = true;

/// Minimum legal encoded instruction length, in bytes (opcode-only forms
/// such as NOP/HALT/RET/FENCE).
pub const MIN_INSTRUCTION_LEN: usize = 1;

/// Maximum legal encoded instruction length, in bytes (MOVI: opcode +
/// register + 8-byte immediate).
pub const MAX_INSTRUCTION_LEN: usize = 10;

/// Opcode space: a single byte, so 256 possible opcodes.
pub const OPCODE_SPACE: usize = 256;

/// Register id encoding occupies one full byte per operand (spec §4: the
/// ISA is dense, but every register field is a whole byte so that decoding
/// is never ambiguous — see docs/ISA_SPEC.md §"Encoding" for the rationale).
/// Only values `0..NUM_GP_REGISTERS` are legal; anything else is
/// `INVALID_REGISTER`.
pub const REGISTER_FIELD_BYTES: usize = 1;

/// Shift/rotate amount is taken from the low 6 bits of the shift-count
/// register (2^6 = 64 covers every bit position of a 64-bit word).
pub const SHIFT_AMOUNT_MASK: u64 = 0x3F;

/// JXCL executable magic bytes (spec §27): ASCII "JXCL".
pub const BINARY_MAGIC: [u8; 4] = *b"JXCL";

/// Current binary format version.
pub const BINARY_VERSION: u16 = 1;

/// Architecture identifier stored in the binary header, distinguishing
/// this ISA revision from any future incompatible one.
pub const ARCHITECTURE_ID: u16 = 0x4A58; // "JX"

/// Default execution step limit used by `run()` to guarantee termination
/// on non-halting programs (spec §13). `run_until_halt` accepts an
/// explicit override.
pub const DEFAULT_EXECUTION_LIMIT: u64 = 10_000_000;
