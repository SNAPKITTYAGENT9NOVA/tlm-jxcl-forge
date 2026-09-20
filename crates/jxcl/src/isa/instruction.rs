// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `DecodedInstruction` (spec §20): the structured result of decoding a
//! byte stream, and the structured input to the encoder.

use crate::isa::opcodes::Mnemonic;
use crate::isa::operand::Operands;

/// A fully decoded (or, prior to encoding, fully specified) instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub mnemonic: Mnemonic,
    pub operands: Operands,
    /// Encoded length in bytes (redundant with `operands.format().len()`,
    /// but carried explicitly so callers never need to recompute it —
    /// spec §20's `DecodedInstruction { ... length ... }`).
    pub length: usize,
}

impl DecodedInstruction {
    pub fn new(mnemonic: Mnemonic, operands: Operands) -> Self {
        DecodedInstruction {
            mnemonic,
            operands,
            length: operands.format().len(),
        }
    }
}
