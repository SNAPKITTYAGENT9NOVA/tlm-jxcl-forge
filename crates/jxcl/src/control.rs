// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Control-flow subsystem (spec §14, §24): branch target calculation and
//! condition evaluation, kept independent of the execution engine so the
//! formulas are auditable in one place.

use crate::isa::flags::Flags;
use crate::isa::opcodes::Mnemonic;

/// Compute a PC-relative branch/call target (spec §14):
/// `target = pc_after_fetch + displacement`, where `pc_after_fetch` is the
/// address of the instruction immediately following the branch
/// (`pc_of_instruction + instruction_length`, per the
/// `PC_POINTS_TO_CURRENT_INSTRUCTION` convention in `isa::constants`).
pub fn branch_target(pc_after_fetch: u64, displacement: i32) -> u64 {
    (pc_after_fetch as i64).wrapping_add(displacement as i64) as u64
}

/// Evaluate a conditional branch's condition from the current flags
/// (spec §24). Unconditional mnemonics (`JMP`, `CALL`) are not handled
/// here — callers branch unconditionally for those.
///
/// Signed-comparison conditions follow the standard flag algebra after a
/// preceding `CMP`/`SUB`: `N != V` means "less than" in two's-complement
/// arithmetic irrespective of the actual bit patterns compared.
pub fn condition_holds(mnemonic: Mnemonic, flags: Flags) -> bool {
    match mnemonic {
        Mnemonic::Jz => flags.z,
        Mnemonic::Jnz => !flags.z,
        Mnemonic::Jc => flags.c,
        Mnemonic::Jnc => !flags.c,
        Mnemonic::Jl => flags.n != flags.v,
        Mnemonic::Jge => flags.n == flags.v,
        Mnemonic::Jg => !flags.z && (flags.n == flags.v),
        Mnemonic::Jle => flags.z || (flags.n != flags.v),
        other => panic!(
            "condition_holds called with non-conditional mnemonic {:?}",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_target_is_relative_to_next_instruction() {
        assert_eq!(branch_target(100, 10), 110);
        assert_eq!(branch_target(100, -10), 90);
    }

    #[test]
    fn signed_less_than_uses_n_xor_v() {
        let f_lt = Flags {
            z: false,
            n: true,
            c: false,
            v: false,
        };
        assert!(condition_holds(Mnemonic::Jl, f_lt));
        let f_ge = Flags {
            z: false,
            n: false,
            c: false,
            v: false,
        };
        assert!(!condition_holds(Mnemonic::Jl, f_ge));
    }
}
