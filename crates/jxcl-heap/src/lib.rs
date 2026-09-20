// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A simple bump allocator operating within an address space, for
//! programs needing dynamic memory.
//!
//! ## Documented minimal scope
//!
//! This is a bump allocator: `alloc` only ever moves a watermark
//! forward, never reuses freed space, and `free` is an intentional
//! no-op (documented, not a bug -- a real free-list allocator is a
//! separate, larger invariant that a future crate can own without
//! breaking this one's API, matching `docs/CRATE_ARCHITECTURE.md`'s
//! "can evolve independently" test). This is sufficient for programs
//! that allocate for the duration of a run and never need to reclaim
//! individual objects, which is the only workload the base JXCL ISA
//! (spec) itself defines any support for (there is no architectural
//! notion of freeing memory -- that would be a userspace/runtime
//! convention on top of the ISA).
//!
//! ## Deviation from `docs/crates.toml`
//!
//! `jxcl-types` is added as a direct dependency, alongside the
//! registry's listed `jxcl-address-space`, `jxcl-load-store`,
//! `jxcl-exceptions` -- needed to name `Address` in this crate's own
//! public API, the same reason `jxcl-cache-model` and `jxcl-stack` add it.
#![forbid(unsafe_code)]

use jxcl_address_space::{AddressSpace, Permission};
use jxcl_load_store::store_u8;
use jxcl_types::Address;

/// Why [`Heap::alloc`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapError {
    /// Not enough room left before the heap's upper bound.
    OutOfMemory,
    /// `align` was zero or not a power of two.
    InvalidAlignment,
    /// The address space doesn't grant read+write over the region this
    /// allocation would occupy (e.g. the heap region wasn't registered,
    /// or was registered without write permission).
    PermissionDenied,
}

/// A watermark-style bump allocator over `[base, base+len)` of an
/// [`AddressSpace`]. Owns only the bookkeeping (the next free address);
/// the bytes themselves live in the `AddressSpace`'s backing memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heap {
    base: Address,
    limit: Address,
    next: Address,
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}

impl Heap {
    /// A heap over `[base, base+len)`. Does not itself register a region
    /// with any `AddressSpace`; pair it with one that already grants
    /// read+write over that range (e.g. via `jxcl-memory-map`).
    pub fn new(base: Address, len: u64) -> Self {
        Heap {
            base,
            limit: Address::new(base.get().saturating_add(len)),
            next: base,
        }
    }

    /// Build a heap from a named region already registered in `space`
    /// (e.g. `jxcl-memory-map::DEFAULT_MEMORY_MAP`'s `"heap"` region).
    pub fn from_region(space: &AddressSpace, region_name: &str) -> Option<Self> {
        space
            .regions()
            .iter()
            .find(|r| r.name == region_name)
            .map(|r| Heap::new(r.base, r.len))
    }

    pub fn base(&self) -> Address {
        self.base
    }
    pub fn limit(&self) -> Address {
        self.limit
    }
    pub fn used(&self) -> u64 {
        self.next.get() - self.base.get()
    }
    pub fn remaining(&self) -> u64 {
        self.limit.get() - self.next.get()
    }

    /// Allocate `size` zero-initialized bytes, aligned to `align` (which
    /// must be a nonzero power of two), inside `space`. Bumps the
    /// watermark forward and never reuses space handed out by an earlier
    /// call, even after [`Heap::free`].
    pub fn alloc(
        &mut self,
        space: &mut AddressSpace,
        size: u64,
        align: u64,
    ) -> Result<Address, HeapError> {
        let aligned_start = align_up(self.next.get(), align).ok_or(HeapError::InvalidAlignment)?;
        let end = aligned_start
            .checked_add(size)
            .ok_or(HeapError::OutOfMemory)?;
        if end > self.limit.get() {
            return Err(HeapError::OutOfMemory);
        }
        let addr = Address::new(aligned_start);
        if size > 0 && !space.is_permitted(addr, size, Permission::READ_WRITE) {
            return Err(HeapError::PermissionDenied);
        }
        for i in 0..size {
            // Zero-initialize; a fresh `Memory` already starts zeroed,
            // but this makes the guarantee explicit and independent of
            // what was previously in this range.
            store_u8(space.memory_mut(), Address::new(aligned_start + i), 0)
                .map_err(|_| HeapError::PermissionDenied)?;
        }
        self.next = Address::new(end);
        Ok(addr)
    }

    /// Intentional no-op: see the module-level "Documented minimal
    /// scope" note. Accepting `addr` (rather than no parameters) keeps
    /// the call site meaningful and ready for a future real
    /// implementation without changing its signature.
    pub fn free(&mut self, _addr: Address) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_address_space::Region;
    use jxcl_memory::Memory;

    fn space_with_heap(len: u64) -> AddressSpace {
        let mut space = AddressSpace::new(Memory::new(len));
        space
            .add_region(Region::new(
                "heap",
                Address::new(0),
                len,
                Permission::READ_WRITE,
            ))
            .unwrap();
        space
    }

    #[test]
    fn sequential_allocations_never_overlap() {
        let mut space = space_with_heap(256);
        let mut heap = Heap::new(Address::new(0), 256);
        let a = heap.alloc(&mut space, 16, 8).unwrap();
        let b = heap.alloc(&mut space, 16, 8).unwrap();
        assert_eq!(a, Address::new(0));
        assert_eq!(b, Address::new(16));
    }

    #[test]
    fn allocations_are_aligned() {
        let mut space = space_with_heap(256);
        let mut heap = Heap::new(Address::new(0), 256);
        heap.alloc(&mut space, 3, 8).unwrap(); // bumps next to 3, next alloc must realign
        let b = heap.alloc(&mut space, 8, 8).unwrap();
        assert_eq!(b.get() % 8, 0);
    }

    #[test]
    fn out_of_memory_when_exceeding_the_heap() {
        let mut space = space_with_heap(16);
        let mut heap = Heap::new(Address::new(0), 16);
        assert_eq!(
            heap.alloc(&mut space, 32, 1).unwrap_err(),
            HeapError::OutOfMemory
        );
    }

    #[test]
    fn invalid_alignment_is_rejected() {
        let mut space = space_with_heap(16);
        let mut heap = Heap::new(Address::new(0), 16);
        assert_eq!(
            heap.alloc(&mut space, 4, 3).unwrap_err(),
            HeapError::InvalidAlignment
        );
    }

    #[test]
    fn permission_denied_when_region_is_read_only() {
        let mut space = AddressSpace::new(Memory::new(16));
        space
            .add_region(Region::new(
                "rodata",
                Address::new(0),
                16,
                Permission::READ_ONLY,
            ))
            .unwrap();
        let mut heap = Heap::new(Address::new(0), 16);
        assert_eq!(
            heap.alloc(&mut space, 4, 1).unwrap_err(),
            HeapError::PermissionDenied
        );
    }

    #[test]
    fn free_is_a_documented_no_op() {
        let mut space = space_with_heap(16);
        let mut heap = Heap::new(Address::new(0), 16);
        let a = heap.alloc(&mut space, 8, 1).unwrap();
        let used_before = heap.used();
        heap.free(a);
        assert_eq!(heap.used(), used_before, "free must not reclaim space");
    }

    #[test]
    fn from_region_finds_the_named_region() {
        let space = space_with_heap(64);
        let heap = Heap::from_region(&space, "heap").unwrap();
        assert_eq!(heap.base(), Address::new(0));
        assert_eq!(heap.remaining(), 64);
        assert!(Heap::from_region(&space, "nonexistent").is_none());
    }

    #[test]
    fn allocated_bytes_are_zeroed() {
        let mut space = space_with_heap(16);
        let mut heap = Heap::new(Address::new(0), 16);
        space.memory_mut().write8(Address::new(0), 0xFF).unwrap();
        let a = heap.alloc(&mut space, 4, 1).unwrap();
        for i in 0..4 {
            assert_eq!(space.memory().read8(Address::new(a.get() + i)).unwrap(), 0);
        }
    }
}
