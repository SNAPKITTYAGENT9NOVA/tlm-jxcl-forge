// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The JXCL instruction-set architecture definition (spec §3-§9, §19-§20).
//!
//! This module tree is the frozen architectural layer: constants, register
//! model, flags, operand/format model and the opcode registry. Everything
//! else in the crate (encoding, execution, assembler, disassembler) is
//! built on top of it and must not redefine any of it independently.

pub mod constants;
pub mod flags;
pub mod instruction;
pub mod opcodes;
pub mod operand;
pub mod registers;
