//! Branch/jump condition evaluation against the flags register.
//!
//! Owns: Condition-code evaluation (branch-taken decisions). Extracted from `jxcl/src/control.rs`.
//!
//! Evaluates conditional branch instructions against the current flags (spec §24).
//! Unconditional mnemonics (JMP, CALL) are not handled here — callers branch
//! unconditionally for those.
//!
//! Signed-comparison conditions follow the standard flag algebra after a
//! preceding CMP/SUB: `N != V` means "less than" in two's-complement
//! arithmetic irrespective of the actual bit patterns compared (spec §3).
#![forbid(unsafe_code)]

use jxcl_flags::Flags;
use jxcl_opcodes::Mnemonic;

/// Evaluate a conditional branch's condition from the current flags (spec §24).
///
/// Returns true if the branch should be taken, false otherwise.
///
/// Handles: JZ (zero), JNZ (not zero), JC (carry), JNC (not carry),
/// JL (less), JGE (greater or equal), JG (greater), JLE (less or equal).
///
/// Signed-comparison conditions follow the standard flag algebra:
/// - JL (less): N != V
/// - JGE (greater or equal): N == V
/// - JG (greater): !Z && (N == V)
/// - JLE (less or equal): Z || (N != V)
pub fn evaluate_condition(mnemonic: Mnemonic, flags: Flags) -> bool {
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
            "evaluate_condition called with non-conditional mnemonic {:?}",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_flag_conditions() {
        let flags_zero = Flags { z: true, n: false, c: false, v: false };
        let flags_nonzero = Flags { z: false, n: false, c: false, v: false };

        assert!(evaluate_condition(Mnemonic::Jz, flags_zero));
        assert!(!evaluate_condition(Mnemonic::Jz, flags_nonzero));
        assert!(!evaluate_condition(Mnemonic::Jnz, flags_zero));
        assert!(evaluate_condition(Mnemonic::Jnz, flags_nonzero));
    }

    #[test]
    fn carry_flag_conditions() {
        let flags_carry = Flags { z: false, n: false, c: true, v: false };
        let flags_no_carry = Flags { z: false, n: false, c: false, v: false };

        assert!(evaluate_condition(Mnemonic::Jc, flags_carry));
        assert!(!evaluate_condition(Mnemonic::Jc, flags_no_carry));
        assert!(!evaluate_condition(Mnemonic::Jnc, flags_carry));
        assert!(evaluate_condition(Mnemonic::Jnc, flags_no_carry));
    }

    #[test]
    fn signed_less_than_uses_n_xor_v() {
        // N=1, V=0: signed less than (true)
        let flags_lt = Flags { z: false, n: true, c: false, v: false };
        assert!(evaluate_condition(Mnemonic::Jl, flags_lt));
        assert!(!evaluate_condition(Mnemonic::Jge, flags_lt));

        // N=0, V=0: not signed less than (false)
        let flags_ge = Flags { z: false, n: false, c: false, v: false };
        assert!(!evaluate_condition(Mnemonic::Jl, flags_ge));
        assert!(evaluate_condition(Mnemonic::Jge, flags_ge));

        // N=1, V=1: not signed less than (they're equal)
        let flags_no_lt = Flags { z: false, n: true, c: false, v: true };
        assert!(!evaluate_condition(Mnemonic::Jl, flags_no_lt));
        assert!(evaluate_condition(Mnemonic::Jge, flags_no_lt));
    }

    #[test]
    fn greater_conditions() {
        // Z=0, N=0, V=0: greater (true)
        let flags_gt = Flags { z: false, n: false, c: false, v: false };
        assert!(evaluate_condition(Mnemonic::Jg, flags_gt));
        assert!(!evaluate_condition(Mnemonic::Jle, flags_gt));

        // Z=1, N=0, V=0: not greater (equal)
        let flags_eq = Flags { z: true, n: false, c: false, v: false };
        assert!(!evaluate_condition(Mnemonic::Jg, flags_eq));
        assert!(evaluate_condition(Mnemonic::Jle, flags_eq));

        // Z=0, N=1, V=0: not greater (less than)
        let flags_lt = Flags { z: false, n: true, c: false, v: false };
        assert!(!evaluate_condition(Mnemonic::Jg, flags_lt));
        assert!(evaluate_condition(Mnemonic::Jle, flags_lt));
    }
}
