//! Named memory regions (code/data/stack) with permission flags layered
//! over raw storage.
//!
//! `jxcl-memory` only knows about bounds and alignment; this crate adds
//! the second half of `crates/jxcl/src/memory.rs`'s design -- a
//! permission model -- but generalizes it from that file's hard-coded
//! two-region (code / everything-else) split into an arbitrary list of
//! named, non-overlapping `Region`s, each with its own `Permission`. An
//! access that isn't entirely covered by exactly one declared region is
//! denied, matching a real MMU's "unmapped is inaccessible" behavior
//! (mirroring `crates/jxcl/src/memory.rs`'s `overlaps` straddling check:
//! an access spanning a region boundary is never partially permitted).
#![forbid(unsafe_code)]

use std::fmt;

use jxcl_memory::Memory;
use jxcl_types::Address;

/// Read/write/execute permission bits a [`Region`] grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Permission {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permission {
    pub const NONE: Permission = Permission {
        read: false,
        write: false,
        execute: false,
    };
    pub const READ_ONLY: Permission = Permission {
        read: true,
        write: false,
        execute: false,
    };
    pub const READ_WRITE: Permission = Permission {
        read: true,
        write: true,
        execute: false,
    };
    pub const READ_EXECUTE: Permission = Permission {
        read: true,
        write: false,
        execute: true,
    };

    pub const fn new(read: bool, write: bool, execute: bool) -> Self {
        Permission {
            read,
            write,
            execute,
        }
    }

    /// True iff `self` grants every bit set in `requested`.
    pub const fn allows(&self, requested: Permission) -> bool {
        (!requested.read || self.read)
            && (!requested.write || self.write)
            && (!requested.execute || self.execute)
    }
}

/// A named, contiguous range of the address space and the permission it grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    pub base: Address,
    pub len: u64,
    pub permission: Permission,
}

impl Region {
    pub fn new(name: impl Into<String>, base: Address, len: u64, permission: Permission) -> Self {
        Region {
            name: name.into(),
            base,
            len,
            permission,
        }
    }

    /// One past the last address in this region (`base + len`, saturating
    /// so a region that reaches the top of the address space doesn't wrap
    /// around to look empty).
    pub fn end(&self) -> Address {
        Address::new(self.base.get().saturating_add(self.len))
    }

    /// True iff `[addr, addr+len)` lies entirely within this region.
    pub fn contains_range(&self, addr: Address, len: u64) -> bool {
        if len == 0 {
            return addr.get() >= self.base.get() && addr.get() <= self.end().get();
        }
        match addr.get().checked_add(len) {
            Some(range_end) => addr.get() >= self.base.get() && range_end <= self.end().get(),
            None => false,
        }
    }

    /// True iff `[addr, addr+len)` overlaps this region at all (used to
    /// reject overlapping regions at `add_region` time).
    fn overlaps(&self, addr: Address, len: u64) -> bool {
        if len == 0 || self.len == 0 {
            return false;
        }
        match addr.get().checked_add(len) {
            Some(range_end) => addr.get() < self.end().get() && self.base.get() < range_end,
            None => true,
        }
    }
}

/// Why [`AddressSpace::add_region`] rejected a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceError {
    /// The region extends past the end of backing memory.
    OutOfBounds,
    /// The region overlaps an already-registered region.
    Overlap,
}

impl fmt::Display for AddressSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressSpaceError::OutOfBounds => write!(f, "region extends past end of memory"),
            AddressSpaceError::Overlap => write!(f, "region overlaps an existing region"),
        }
    }
}
impl std::error::Error for AddressSpaceError {}

/// Raw storage plus the set of named, permissioned regions layered over it.
#[derive(Debug)]
pub struct AddressSpace {
    memory: Memory,
    regions: Vec<Region>,
}

impl AddressSpace {
    pub fn new(memory: Memory) -> Self {
        AddressSpace {
            memory,
            regions: Vec::new(),
        }
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// Register a region. Rejects a region that runs past the end of
    /// backing memory or overlaps a region already registered.
    pub fn add_region(&mut self, region: Region) -> Result<(), AddressSpaceError> {
        if region.end().get() > self.memory.len() {
            return Err(AddressSpaceError::OutOfBounds);
        }
        if self
            .regions
            .iter()
            .any(|r| r.overlaps(region.base, region.len))
        {
            return Err(AddressSpaceError::Overlap);
        }
        self.regions.push(region);
        Ok(())
    }

    /// True iff `[addr, addr+len)` is entirely covered by one registered
    /// region whose permission grants every bit of `requested`. An access
    /// that falls outside every region, or straddles a region boundary,
    /// is denied.
    pub fn is_permitted(&self, addr: Address, len: u64, requested: Permission) -> bool {
        self.regions
            .iter()
            .find(|r| r.contains_range(addr, len))
            .is_some_and(|r| r.permission.allows(requested))
    }

    /// Find the region (if any) that entirely covers `[addr, addr+len)`.
    pub fn region_containing(&self, addr: Address, len: u64) -> Option<&Region> {
        self.regions.iter().find(|r| r.contains_range(addr, len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(v: u64) -> Address {
        Address::new(v)
    }

    #[test]
    fn add_region_within_bounds_succeeds() {
        let mut sp = AddressSpace::new(Memory::new(64));
        assert!(sp
            .add_region(Region::new("code", addr(0), 16, Permission::READ_EXECUTE))
            .is_ok());
        assert_eq!(sp.regions().len(), 1);
    }

    #[test]
    fn add_region_past_end_of_memory_is_rejected() {
        let mut sp = AddressSpace::new(Memory::new(16));
        assert_eq!(
            sp.add_region(Region::new("oops", addr(8), 16, Permission::READ_WRITE))
                .unwrap_err(),
            AddressSpaceError::OutOfBounds
        );
    }

    #[test]
    fn overlapping_regions_are_rejected() {
        let mut sp = AddressSpace::new(Memory::new(64));
        sp.add_region(Region::new("a", addr(0), 16, Permission::READ_WRITE))
            .unwrap();
        assert_eq!(
            sp.add_region(Region::new("b", addr(8), 16, Permission::READ_WRITE))
                .unwrap_err(),
            AddressSpaceError::Overlap
        );
        // Adjacent, non-overlapping is fine.
        sp.add_region(Region::new("c", addr(16), 16, Permission::READ_WRITE))
            .unwrap();
    }

    #[test]
    fn is_permitted_checks_the_requested_bits() {
        let mut sp = AddressSpace::new(Memory::new(64));
        sp.add_region(Region::new("code", addr(0), 16, Permission::READ_EXECUTE))
            .unwrap();
        assert!(sp.is_permitted(addr(0), 4, Permission::READ_ONLY));
        assert!(!sp.is_permitted(addr(0), 4, Permission::READ_WRITE));
        assert!(sp.is_permitted(addr(4), 1, Permission::new(false, false, true)));
    }

    #[test]
    fn unmapped_access_is_denied_by_default() {
        let sp = AddressSpace::new(Memory::new(64));
        assert!(!sp.is_permitted(addr(0), 4, Permission::READ_ONLY));
    }

    #[test]
    fn straddling_access_across_two_regions_is_denied() {
        let mut sp = AddressSpace::new(Memory::new(64));
        sp.add_region(Region::new("a", addr(0), 8, Permission::READ_WRITE))
            .unwrap();
        sp.add_region(Region::new("b", addr(8), 8, Permission::READ_WRITE))
            .unwrap();
        // [4, 12) spans both regions -- no single region covers it.
        assert!(!sp.is_permitted(addr(4), 8, Permission::READ_ONLY));
    }

    #[test]
    fn region_containing_finds_the_right_region() {
        let mut sp = AddressSpace::new(Memory::new(64));
        sp.add_region(Region::new("stack", addr(32), 32, Permission::READ_WRITE))
            .unwrap();
        let found = sp.region_containing(addr(40), 4).unwrap();
        assert_eq!(found.name, "stack");
        assert!(sp.region_containing(addr(0), 4).is_none());
    }
}
