//! ISA-wide constants (spec §3, §10, §17, §27, §40), extracted verbatim
//! from `jxcl/src/isa/constants.rs`.
//!
//! These values are the frozen architectural parameters of TLM JXCL.
//! Nothing outside this file may silently redefine them (spec §3: "Do not
//! silently change widths").
#![forbid(unsafe_code)]

/// Width, in bits, of every general-purpose register and of PC/SP/FP.
pub const WORD_BITS: u32 = 64;

/// Kept as an alias of [`WORD_BITS`] for readers coming from the
/// pre-expansion name (`jxcl::isa::constants::REGISTER_WIDTH_BITS`).
pub const REGISTER_WIDTH_BITS: u32 = WORD_BITS;

/// Number of architectural general-purpose registers (R0..=R31).
pub const REGISTER_COUNT: usize = 32;

/// Kept as an alias of [`REGISTER_COUNT`] for readers coming from the
/// pre-expansion name.
pub const NUM_GP_REGISTERS: usize = REGISTER_COUNT;

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

/// Width, in bits, of the opcode field (always the first byte of every
/// instruction). Named to match this crate's `docs/crates.toml`
/// `public_api` entry (`OPCODE_BITS`); equal to `OPCODE_SPACE.ilog2()`.
pub const OPCODE_BITS: u32 = 8;

/// Deviation note: `docs/crates.toml` lists `MEMORY_SIZE` in this
/// crate's `public_api`, but the pre-expansion `jxcl` crate has no such
/// constant -- `main.rs`'s `run` subcommand computes machine memory
/// per-program as `code_size + data_size + headroom`, where `headroom`
/// is this literal `1 << 20` (1 MiB) inlined at the call site. Total
/// addressable *memory* sizing is properly owned by `jxcl-memory-map`
/// (a "memory" category crate outside this batch), so rather than
/// duplicate a value that crate will own, this constant documents only
/// the one real number that existed pre-expansion: the CLI's default
/// stack/heap headroom above a program's code+data.
pub const DEFAULT_MEMORY_HEADROOM: usize = 1 << 20;

/// Kept as an alias matching `docs/crates.toml`'s literal `public_api`
/// name; see [`DEFAULT_MEMORY_HEADROOM`]'s doc comment for what this
/// actually measures pre-expansion (there is no single "total memory
/// size" constant to extract yet -- that's `jxcl-memory-map`'s job).
pub const MEMORY_SIZE: usize = DEFAULT_MEMORY_HEADROOM;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_and_address_widths_match_a_64_bit_machine() {
        assert_eq!(WORD_BITS, 64);
        assert_eq!(ADDRESS_WIDTH_BITS, 64);
    }

    #[test]
    fn register_count_and_field_width_are_consistent() {
        // A byte-wide register field can name up to 256 registers;
        // the ISA only uses 32 of them, but the field must be able to.
        const { assert!(REGISTER_COUNT <= 1 << (REGISTER_FIELD_BYTES * 8)) };
        assert_eq!(REGISTER_COUNT, 32);
    }

    #[test]
    fn opcode_space_matches_a_single_byte() {
        assert_eq!(OPCODE_SPACE, 1usize << OPCODE_BITS);
        assert_eq!(OPCODE_SPACE, 256);
    }

    #[test]
    fn instruction_length_bounds_are_sane() {
        const { assert!(MIN_INSTRUCTION_LEN <= MAX_INSTRUCTION_LEN) };
        assert_eq!(MIN_INSTRUCTION_LEN, 1);
        assert_eq!(MAX_INSTRUCTION_LEN, 10);
    }

    #[test]
    fn shift_amount_mask_covers_every_bit_of_a_word() {
        // 2^6 = 64, i.e. the mask can select any bit position 0..64.
        assert_eq!(SHIFT_AMOUNT_MASK, 0x3F);
        assert_eq!(SHIFT_AMOUNT_MASK as u32 + 1, WORD_BITS);
    }

    #[test]
    fn binary_magic_is_ascii_jxcl() {
        assert_eq!(&BINARY_MAGIC, b"JXCL");
    }

    #[test]
    fn legacy_name_aliases_match_current_names() {
        assert_eq!(REGISTER_WIDTH_BITS, WORD_BITS);
        assert_eq!(NUM_GP_REGISTERS, REGISTER_COUNT);
    }
}
