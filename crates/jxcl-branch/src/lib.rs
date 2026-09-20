// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Branch-target address computation (relative/absolute addressing) split out of condition evaluation.
//!
//! Owns: compute_branch_target. Extracted from `jxcl/src/control.rs`.
//!
//! Handles PC-relative branch/call target calculation:
//! target = pc_after_fetch + displacement (spec §14)
//!
//! where `pc_after_fetch` is the address of the instruction immediately
//! following the branch (pc_of_instruction + instruction_length, per the
//! PC_POINTS_TO_CURRENT_INSTRUCTION convention).
#![forbid(unsafe_code)]

use jxcl_types::Word;

/// Compute a PC-relative branch/call target (spec §14).
///
/// The target is calculated as:
/// `target = pc_after_fetch + displacement`
///
/// where `pc_after_fetch` is the address of the instruction immediately
/// following the branch instruction, and `displacement` is a signed offset
/// from the next instruction.
pub fn compute_branch_target(pc_after_fetch: Word, displacement: i32) -> Word {
    let pc_signed = pc_after_fetch.as_i64();
    let target = pc_signed.wrapping_add(displacement as i64);
    Word::new(target as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_target_is_relative_to_next_instruction() {
        let pc = Word::new(100);
        assert_eq!(compute_branch_target(pc, 10), Word::new(110));
        assert_eq!(compute_branch_target(pc, -10), Word::new(90));
    }

    #[test]
    fn branch_target_wraps_on_overflow() {
        let pc = Word::MAX;
        let result = compute_branch_target(pc, 1);
        assert_eq!(result, Word::ZERO);
    }

    #[test]
    fn branch_target_wraps_on_underflow() {
        let pc = Word::ZERO;
        let result = compute_branch_target(pc, -1);
        assert_eq!(result, Word::MAX);
    }

    #[test]
    fn branch_target_with_large_positive_offset() {
        let pc = Word::new(0x100000);
        let result = compute_branch_target(pc, 0x10000);
        assert_eq!(result, Word::new(0x110000));
    }

    #[test]
    fn branch_target_with_large_negative_offset() {
        let pc = Word::new(0x200000);
        let result = compute_branch_target(pc, -0x100000);
        assert_eq!(result, Word::new(0x100000));
    }

    #[test]
    fn branch_target_zero_displacement() {
        let pc = Word::new(42);
        assert_eq!(compute_branch_target(pc, 0), pc);
    }
}
