// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Register model (spec §5): the architectural register file.

use crate::errors::ExecutionFault;
use crate::isa::constants::{NUM_GP_REGISTERS, R0_HARDWIRED_ZERO};

/// Identifies a general-purpose register operand (`0..NUM_GP_REGISTERS`).
pub type RegId = u8;

/// Validate that `id` names an architectural general-purpose register.
pub fn validate_register(id: RegId) -> Result<(), ExecutionFault> {
    if (id as usize) < NUM_GP_REGISTERS {
        Ok(())
    } else {
        Err(ExecutionFault::InvalidRegister)
    }
}

/// The complete architectural register file: 32 general-purpose registers
/// plus the special registers PC, SP, FP and FLAGS (spec §3, §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterFile {
    gp: [u64; NUM_GP_REGISTERS],
    pub pc: u64,
    pub sp: u64,
    pub fp: u64,
    /// Raw flags word; see `crate::isa::flags::Flags` for the bit layout.
    pub flags: u64,
}

impl Default for RegisterFile {
    fn default() -> Self {
        RegisterFile {
            gp: [0; NUM_GP_REGISTERS],
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
    pub fn read(&self, id: RegId) -> Result<u64, ExecutionFault> {
        validate_register(id)?;
        if R0_HARDWIRED_ZERO && id == 0 {
            return Ok(0);
        }
        Ok(self.gp[id as usize])
    }

    /// Write a general-purpose register. Writes to R0 are discarded when
    /// `R0_HARDWIRED_ZERO` is set (spec §5).
    pub fn write(&mut self, id: RegId, value: u64) -> Result<(), ExecutionFault> {
        validate_register(id)?;
        if R0_HARDWIRED_ZERO && id == 0 {
            return Ok(());
        }
        self.gp[id as usize] = value;
        Ok(())
    }

    /// Raw view of all 32 general-purpose registers, for snapshotting/tracing.
    pub fn general_registers(&self) -> &[u64; NUM_GP_REGISTERS] {
        &self.gp
    }

    pub fn set_general_registers(&mut self, values: [u64; NUM_GP_REGISTERS]) {
        self.gp = values;
        if R0_HARDWIRED_ZERO {
            self.gp[0] = 0;
        }
    }
}
