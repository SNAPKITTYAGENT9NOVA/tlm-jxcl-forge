//! Typed, sign-extension-aware load/store helpers (u8/u16/u32/u64 and
//! signed variants) between raw memory and the execution engine.
//!
//! This is the layer `crates/jxcl/src/execution.rs` calls directly
//! (`state.memory.read64`/`write64`, wrapped in its `try_mem!` macro that
//! turns a `MemoryFault` into an `ExecutionFault`); this crate is that
//! same "typed access + fault conversion" pattern, generalized to every
//! width and to signed values, and converting into `jxcl-exceptions`'s
//! shared `Exception` type instead of a bespoke one.
#![forbid(unsafe_code)]

use jxcl_exceptions::Exception;
use jxcl_memory::Memory;
use jxcl_types::Address;

/// Load an unsigned 8-bit value.
pub fn load_u8(mem: &Memory, addr: Address) -> Result<u8, Exception> {
    Ok(mem.read8(addr)?)
}
/// Load an unsigned 16-bit value.
pub fn load_u16(mem: &Memory, addr: Address) -> Result<u16, Exception> {
    Ok(mem.read16(addr)?)
}
/// Load an unsigned 32-bit value.
pub fn load_u32(mem: &Memory, addr: Address) -> Result<u32, Exception> {
    Ok(mem.read32(addr)?)
}
/// Load an unsigned 64-bit value.
pub fn load_u64(mem: &Memory, addr: Address) -> Result<u64, Exception> {
    Ok(mem.read64(addr)?)
}

/// Load an 8-bit value, sign-extended to 64 bits.
pub fn load_i8(mem: &Memory, addr: Address) -> Result<i64, Exception> {
    Ok(mem.read8(addr)? as i8 as i64)
}
/// Load a 16-bit value, sign-extended to 64 bits.
pub fn load_i16(mem: &Memory, addr: Address) -> Result<i64, Exception> {
    Ok(mem.read16(addr)? as i16 as i64)
}
/// Load a 32-bit value, sign-extended to 64 bits.
pub fn load_i32(mem: &Memory, addr: Address) -> Result<i64, Exception> {
    Ok(mem.read32(addr)? as i32 as i64)
}
/// Load a 64-bit value, reinterpreted as signed.
pub fn load_i64(mem: &Memory, addr: Address) -> Result<i64, Exception> {
    Ok(mem.read64(addr)? as i64)
}

/// Store an unsigned 8-bit value.
pub fn store_u8(mem: &mut Memory, addr: Address, value: u8) -> Result<(), Exception> {
    Ok(mem.write8(addr, value)?)
}
/// Store an unsigned 16-bit value.
pub fn store_u16(mem: &mut Memory, addr: Address, value: u16) -> Result<(), Exception> {
    Ok(mem.write16(addr, value)?)
}
/// Store an unsigned 32-bit value.
pub fn store_u32(mem: &mut Memory, addr: Address, value: u32) -> Result<(), Exception> {
    Ok(mem.write32(addr, value)?)
}
/// Store an unsigned 64-bit value.
pub fn store_u64(mem: &mut Memory, addr: Address, value: u64) -> Result<(), Exception> {
    Ok(mem.write64(addr, value)?)
}

/// Store a signed value, truncated to 8 bits.
pub fn store_i8(mem: &mut Memory, addr: Address, value: i64) -> Result<(), Exception> {
    Ok(mem.write8(addr, value as u8)?)
}
/// Store a signed value, truncated to 16 bits.
pub fn store_i16(mem: &mut Memory, addr: Address, value: i64) -> Result<(), Exception> {
    Ok(mem.write16(addr, value as u16)?)
}
/// Store a signed value, truncated to 32 bits.
pub fn store_i32(mem: &mut Memory, addr: Address, value: i64) -> Result<(), Exception> {
    Ok(mem.write32(addr, value as u32)?)
}
/// Store a signed value at full 64-bit width.
pub fn store_i64(mem: &mut Memory, addr: Address, value: i64) -> Result<(), Exception> {
    Ok(mem.write64(addr, value as u64)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_errors::MemoryFault;

    fn addr(v: u64) -> Address {
        Address::new(v)
    }

    #[test]
    fn unsigned_roundtrip_every_width() {
        let mut mem = Memory::new(64);
        store_u8(&mut mem, addr(0), 0xAB).unwrap();
        assert_eq!(load_u8(&mem, addr(0)).unwrap(), 0xAB);
        store_u16(&mut mem, addr(2), 0xBEEF).unwrap();
        assert_eq!(load_u16(&mem, addr(2)).unwrap(), 0xBEEF);
        store_u32(&mut mem, addr(4), 0xDEAD_BEEF).unwrap();
        assert_eq!(load_u32(&mem, addr(4)).unwrap(), 0xDEAD_BEEF);
        store_u64(&mut mem, addr(8), u64::MAX).unwrap();
        assert_eq!(load_u64(&mem, addr(8)).unwrap(), u64::MAX);
    }

    #[test]
    fn signed_loads_sign_extend_correctly() {
        let mut mem = Memory::new(64);
        store_u8(&mut mem, addr(0), 0xFF).unwrap(); // -1 as i8
        assert_eq!(load_i8(&mem, addr(0)).unwrap(), -1);
        store_u16(&mut mem, addr(2), 0x8000).unwrap(); // i16::MIN
        assert_eq!(load_i16(&mem, addr(2)).unwrap(), i16::MIN as i64);
        store_u32(&mut mem, addr(4), 0x8000_0000).unwrap(); // i32::MIN
        assert_eq!(load_i32(&mem, addr(4)).unwrap(), i32::MIN as i64);
        store_u64(&mut mem, addr(8), u64::MAX).unwrap(); // -1i64
        assert_eq!(load_i64(&mem, addr(8)).unwrap(), -1i64);
    }

    #[test]
    fn signed_store_truncates() {
        let mut mem = Memory::new(16);
        store_i8(&mut mem, addr(0), -1).unwrap();
        assert_eq!(load_u8(&mem, addr(0)).unwrap(), 0xFF);
        store_i16(&mut mem, addr(2), -2).unwrap();
        assert_eq!(load_u16(&mem, addr(2)).unwrap(), 0xFFFE);
        store_i32(&mut mem, addr(4), -3).unwrap();
        assert_eq!(load_u32(&mem, addr(4)).unwrap(), 0xFFFF_FFFD);
    }

    #[test]
    fn out_of_bounds_becomes_an_exception() {
        let mem = Memory::new(4);
        let err = load_u64(&mem, addr(0)).unwrap_err();
        assert_eq!(err, Exception::Memory(MemoryFault::InvalidAddress));
    }

    #[test]
    fn misalignment_becomes_an_exception() {
        let mem = Memory::new(16);
        let err = load_u32(&mem, addr(1)).unwrap_err();
        assert_eq!(err, Exception::Memory(MemoryFault::AlignmentFault));
    }

    #[test]
    fn property_roundtrip_across_many_values() {
        let mut mem = Memory::new(4096);
        for (i, v) in [0u64, 1, 42, 0xFFFF, u32::MAX as u64, u64::MAX]
            .iter()
            .enumerate()
        {
            let a = addr((i as u64) * 8);
            store_u64(&mut mem, a, *v).unwrap();
            assert_eq!(load_u64(&mem, a).unwrap(), *v);
        }
        for (i, v) in [0i64, -1, i32::MIN as i64, i32::MAX as i64]
            .iter()
            .enumerate()
        {
            let a = addr(2048 + (i as u64) * 8);
            store_i64(&mut mem, a, *v).unwrap();
            assert_eq!(load_i64(&mem, a).unwrap(), *v);
        }
    }
}
