//! The register file model: indices, names, widths.
//!
//! Extracted from `jxcl/src/isa/registers.rs`. The pre-expansion source
//! represents a register id as a bare `RegId = u8` and validates it
//! against `jxcl::isa::constants::NUM_GP_REGISTERS`/`R0_HARDWIRED_ZERO`.
//! This crate keeps that exact representation and semantics, adapted to
//! depend on `jxcl-types::RegisterIndex` (the workspace-wide newtype for
//! register indices) and `jxcl-constants::{REGISTER_COUNT, R0_HARDWIRED_ZERO}`
//! (the single authoritative source for those parameters) instead of a
//! bare `u8` and locally duplicated constants.
//!
//! Owns: the `Register`/register-index type and the `RegisterFile` model
//! (32 general-purpose registers plus PC/SP/FP/FLAGS, spec §3, §5).
//!
//! ## Deviation from the pre-expansion source
//!
//! `jxcl/src/isa/registers.rs`'s `validate_register`/`RegisterFile::read`/
//! `write` return `crate::errors::ExecutionFault`. Per `docs/crates.toml`
//! this crate depends only on `jxcl-types` and `jxcl-constants` (not
//! `jxcl-errors`), so out-of-range register access here is reported with
//! a small crate-local [`RegisterOutOfRange`] error instead of pulling in
//! `jxcl-errors` for one variant; execution-layer crates that already
//! depend on `jxcl-errors` (e.g. a future `jxcl-machine`) are expected to
//! convert it to `ExecutionFault::InvalidRegister` at their boundary.
#![forbid(unsafe_code)]

use std::fmt;

use jxcl_constants::{R0_HARDWIRED_ZERO, REGISTER_COUNT};
use jxcl_types::RegisterIndex;

/// Identifies a general-purpose register operand (`0..REGISTER_COUNT`).
///
/// Alias kept alongside [`RegisterIndex`] to match `docs/crates.toml`'s
/// `public_api` naming (`Register`) while the real value representation
/// stays `jxcl-types::RegisterIndex`, the workspace-wide newtype.
pub type Register = RegisterIndex;

/// A register index named outside `0..REGISTER_COUNT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterOutOfRange {
    pub index: u8,
}

impl fmt::Display for RegisterOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "register index {} is outside 0..{}",
            self.index, REGISTER_COUNT
        )
    }
}
impl std::error::Error for RegisterOutOfRange {}

/// Validate that `id` names an architectural general-purpose register
/// (spec §5). This is the exact check `jxcl/src/isa/registers.rs`'s
/// `validate_register` performed, adapted to work over `RegisterIndex`
/// and this crate's local error type (see the module-level "Deviation"
/// note).
pub fn validate_register(id: Register) -> Result<(), RegisterOutOfRange> {
    if (id.get() as usize) < REGISTER_COUNT {
        Ok(())
    } else {
        Err(RegisterOutOfRange { index: id.get() })
    }
}

/// The complete architectural register file: `REGISTER_COUNT`
/// general-purpose registers plus the special registers PC, SP, FP and
/// FLAGS (spec §3, §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterFile {
    gp: [u64; REGISTER_COUNT],
    pub pc: u64,
    pub sp: u64,
    pub fp: u64,
    /// Raw flags word; see `jxcl-flags::Flags` for the bit layout.
    pub flags: u64,
}

impl Default for RegisterFile {
    fn default() -> Self {
        RegisterFile {
            gp: [0; REGISTER_COUNT],
            pc: 0,
            sp: 0,
            fp: 0,
            flags: 0,
        }
    }
}

impl RegisterFile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read a general-purpose register. R0 always reads as zero when
    /// `R0_HARDWIRED_ZERO` is set (spec §5).
    pub fn read(&self, id: Register) -> Result<u64, RegisterOutOfRange> {
        validate_register(id)?;
        if R0_HARDWIRED_ZERO && id.get() == 0 {
            return Ok(0);
        }
        Ok(self.gp[id.get() as usize])
    }

    /// Write a general-purpose register. Writes to R0 are discarded when
    /// `R0_HARDWIRED_ZERO` is set (spec §5).
    pub fn write(&mut self, id: Register, value: u64) -> Result<(), RegisterOutOfRange> {
        validate_register(id)?;
        if R0_HARDWIRED_ZERO && id.get() == 0 {
            return Ok(());
        }
        self.gp[id.get() as usize] = value;
        Ok(())
    }

    /// Raw view of all general-purpose registers, for snapshotting/tracing.
    pub fn general_registers(&self) -> &[u64; REGISTER_COUNT] {
        &self.gp
    }

    pub fn set_general_registers(&mut self, values: [u64; REGISTER_COUNT]) {
        self.gp = values;
        if R0_HARDWIRED_ZERO {
            self.gp[0] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_register_accepts_in_range() {
        assert!(validate_register(Register::new(0)).is_ok());
        assert!(validate_register(Register::new((REGISTER_COUNT - 1) as u8)).is_ok());
    }

    #[test]
    fn validate_register_rejects_out_of_range() {
        assert_eq!(
            validate_register(Register::new(REGISTER_COUNT as u8)),
            Err(RegisterOutOfRange {
                index: REGISTER_COUNT as u8
            })
        );
        assert_eq!(
            validate_register(Register::new(255)),
            Err(RegisterOutOfRange { index: 255 })
        );
    }

    #[test]
    fn r0_reads_as_zero_even_after_write() {
        let mut rf = RegisterFile::new();
        rf.write(Register::new(0), 0xdead_beef).unwrap();
        assert_eq!(rf.read(Register::new(0)).unwrap(), 0);
    }

    #[test]
    fn general_purpose_register_roundtrips() {
        let mut rf = RegisterFile::new();
        rf.write(Register::new(5), 42).unwrap();
        assert_eq!(rf.read(Register::new(5)).unwrap(), 42);
    }

    #[test]
    fn read_out_of_range_register_errors() {
        let rf = RegisterFile::new();
        assert_eq!(
            rf.read(Register::new(200)),
            Err(RegisterOutOfRange { index: 200 })
        );
    }

    #[test]
    fn write_out_of_range_register_errors() {
        let mut rf = RegisterFile::new();
        assert_eq!(
            rf.write(Register::new(200), 1),
            Err(RegisterOutOfRange { index: 200 })
        );
    }

    #[test]
    fn register_out_of_range_display() {
        let e = RegisterOutOfRange { index: 40 };
        assert_eq!(e.to_string(), "register index 40 is outside 0..32");
    }

    #[test]
    fn set_general_registers_forces_r0_to_zero() {
        let mut rf = RegisterFile::new();
        let mut values = [7u64; REGISTER_COUNT];
        values[0] = 999;
        rf.set_general_registers(values);
        assert_eq!(rf.general_registers()[0], 0);
        assert_eq!(rf.general_registers()[1], 7);
    }

    #[test]
    fn special_registers_default_to_zero() {
        let rf = RegisterFile::new();
        assert_eq!(rf.pc, 0);
        assert_eq!(rf.sp, 0);
        assert_eq!(rf.fp, 0);
        assert_eq!(rf.flags, 0);
    }
}
