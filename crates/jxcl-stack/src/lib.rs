//! Typed stack-pointer push/pop with overflow/underflow checking for
//! call/return semantics.
//!
//! Mirrors `crates/jxcl/src/execution.rs`'s `PUSH`/`POP`/`CALL`/`RET`
//! bounds checks (`sp < 8` -> `StackFault`, `sp + 8 > memory.len()` ->
//! `StackFault`) but generalized to an arbitrary, explicit `[low, high)`
//! stack region (so the stack need not start at address 0 / end at the
//! top of all of memory, matching `jxcl-memory-map`'s named stack
//! region) and to every stack-cell width via `jxcl-load-store`, with the
//! two distinct fault kinds `jxcl-exceptions::Exception` already
//! distinguishes (`StackOverflow`/`StackUnderflow`) instead of one
//! generic `StackFault`.
#![forbid(unsafe_code)]

use jxcl_exceptions::Exception;
use jxcl_load_store::{load_u64, store_u64};
use jxcl_memory::Memory;
use jxcl_registers::RegisterFile;
use jxcl_types::Address;

/// A downward-growing stack occupying `[low, high)` of the address
/// space. `RegisterFile::sp` (spec §15: PUSH decrements SP before
/// writing, POP reads then increments SP) is the live stack pointer;
/// this type only carries the region bounds and the push/pop logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    low: Address,
    high: Address,
}

impl Stack {
    /// `high` is exclusive (one past the last usable address) and is
    /// also the value `sp` should be initialized to for an empty stack,
    /// matching `crates/jxcl/src/machine.rs`'s `MachineState::new`
    /// convention of starting `sp` at `memory.len()`.
    pub fn new(low: Address, high: Address) -> Self {
        Stack { low, high }
    }

    pub fn low(&self) -> Address {
        self.low
    }
    pub fn high(&self) -> Address {
        self.high
    }

    /// Number of bytes currently pushed (`high - sp`).
    pub fn depth(&self, regs: &RegisterFile) -> u64 {
        self.high.get().saturating_sub(regs.sp)
    }

    pub fn is_empty(&self, regs: &RegisterFile) -> bool {
        regs.sp >= self.high.get()
    }

    /// Push an 8-byte value: decrement `sp` by 8, then write. Faults with
    /// `Exception::StackOverflow` (without writing or moving `sp`) if
    /// that would move `sp` below `low`.
    pub fn push_u64(
        &self,
        regs: &mut RegisterFile,
        mem: &mut Memory,
        value: u64,
    ) -> Result<(), Exception> {
        let new_sp = regs
            .sp
            .checked_sub(8)
            .filter(|&s| s >= self.low.get())
            .ok_or(Exception::StackOverflow)?;
        store_u64(mem, Address::new(new_sp), value)?;
        regs.sp = new_sp;
        Ok(())
    }

    /// Pop an 8-byte value: read at `sp`, then increment `sp` by 8.
    /// Faults with `Exception::StackUnderflow` (without moving `sp`) if
    /// there are fewer than 8 bytes left below `high`.
    pub fn pop_u64(&self, regs: &mut RegisterFile, mem: &Memory) -> Result<u64, Exception> {
        let next_sp = regs
            .sp
            .checked_add(8)
            .filter(|&s| s <= self.high.get())
            .ok_or(Exception::StackUnderflow)?;
        let value = load_u64(mem, Address::new(regs.sp))?;
        regs.sp = next_sp;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(low: u64, high: u64, mem_len: u64) -> (Stack, RegisterFile, Memory) {
        let stack = Stack::new(Address::new(low), Address::new(high));
        let mut regs = RegisterFile::new();
        regs.sp = high;
        (stack, regs, Memory::new(mem_len))
    }

    #[test]
    fn push_then_pop_roundtrips() {
        let (stack, mut regs, mut mem) = setup(0, 64, 64);
        stack.push_u64(&mut regs, &mut mem, 0xDEAD_BEEF).unwrap();
        assert_eq!(regs.sp, 56);
        let v = stack.pop_u64(&mut regs, &mem).unwrap();
        assert_eq!(v, 0xDEAD_BEEF);
        assert_eq!(regs.sp, 64);
    }

    #[test]
    fn push_pop_is_lifo() {
        let (stack, mut regs, mut mem) = setup(0, 64, 64);
        stack.push_u64(&mut regs, &mut mem, 1).unwrap();
        stack.push_u64(&mut regs, &mut mem, 2).unwrap();
        assert_eq!(stack.pop_u64(&mut regs, &mem).unwrap(), 2);
        assert_eq!(stack.pop_u64(&mut regs, &mem).unwrap(), 1);
    }

    #[test]
    fn push_below_low_bound_overflows_without_moving_sp() {
        let (stack, mut regs, mut mem) = setup(56, 64, 64);
        stack.push_u64(&mut regs, &mut mem, 1).unwrap(); // sp: 64 -> 56, exactly at low
        let err = stack.push_u64(&mut regs, &mut mem, 2).unwrap_err();
        assert_eq!(err, Exception::StackOverflow);
        assert_eq!(regs.sp, 56, "sp must not move on a failed push");
    }

    #[test]
    fn pop_past_high_bound_underflows_without_moving_sp() {
        let (stack, mut regs, mem) = setup(0, 64, 64);
        // sp already at `high` (empty stack).
        let err = stack.pop_u64(&mut regs, &mem).unwrap_err();
        assert_eq!(err, Exception::StackUnderflow);
        assert_eq!(regs.sp, 64);
    }

    #[test]
    fn depth_and_is_empty_track_pushes() {
        let (stack, mut regs, mut mem) = setup(0, 64, 64);
        assert!(stack.is_empty(&regs));
        assert_eq!(stack.depth(&regs), 0);
        stack.push_u64(&mut regs, &mut mem, 1).unwrap();
        assert!(!stack.is_empty(&regs));
        assert_eq!(stack.depth(&regs), 8);
    }
}
