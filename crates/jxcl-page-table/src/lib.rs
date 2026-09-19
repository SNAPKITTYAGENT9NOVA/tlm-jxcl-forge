//! A single-level virtual-to-physical page table with map/unmap/translate
//! and page-fault on unmapped access.
//!
//! There is no pre-expansion source for this -- the base JXCL ISA (spec)
//! has no virtual memory at all, only `crates/jxcl/src/memory.rs`'s flat
//! physical address space. This crate is new functionality: a real,
//! testable single-level page table that a future MMU-aware execution
//! mode could sit in front of that flat space.
//!
//! ## Documented scope
//!
//! - **Page size** defaults to 4096 bytes (a conventional, arbitrary
//!   choice -- see [`DEFAULT_PAGE_SIZE`]) and must be a power of two.
//! - **Single-level**: one flat map from virtual page number to physical
//!   page number, not a multi-level radix tree. This is the "single-level"
//!   the registry's purpose calls for; a multi-level table is a distinct,
//!   larger invariant left to a possible future crate.
//! - [`PageTable::translate_checked`] only translates an access that
//!   stays within one virtual page; an access spanning two pages returns
//!   `Exception::MisalignedAccess` rather than silently stitching
//!   together two (possibly non-contiguous) physical ranges.
#![forbid(unsafe_code)]

use std::collections::HashMap;

use jxcl_address_space::{AddressSpace, Permission};
use jxcl_exceptions::Exception;
use jxcl_types::Address;

/// The default page size: 4096 bytes, a conventional (if architecturally
/// arbitrary) choice documented here since the base ISA doesn't specify one.
pub const DEFAULT_PAGE_SIZE: u64 = 4096;

/// Why constructing a [`PageTable`] or calling [`PageTable::map`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableError {
    /// The page size was zero or not a power of two.
    InvalidPageSize,
    /// A virtual or physical address passed to `map` wasn't page-aligned.
    Unaligned,
}

/// A single-level virtual-to-physical page table: one flat mapping from
/// virtual page number (VPN) to physical page number (PPN).
#[derive(Debug, Clone)]
pub struct PageTable {
    page_size: u64,
    entries: HashMap<u64, u64>,
}

impl PageTable {
    /// A page table using [`DEFAULT_PAGE_SIZE`].
    pub fn new() -> Self {
        Self::with_page_size(DEFAULT_PAGE_SIZE).expect("DEFAULT_PAGE_SIZE is a power of two")
    }

    pub fn with_page_size(page_size: u64) -> Result<Self, PageTableError> {
        if page_size == 0 || !page_size.is_power_of_two() {
            return Err(PageTableError::InvalidPageSize);
        }
        Ok(PageTable {
            page_size,
            entries: HashMap::new(),
        })
    }

    pub fn page_size(&self) -> u64 {
        self.page_size
    }

    fn page_number(&self, addr: Address) -> u64 {
        addr.get() / self.page_size
    }

    /// Map the page containing `vaddr` to the page containing `paddr`.
    /// Both must be page-aligned. Overwrites any existing mapping for
    /// that virtual page, returning the previous physical page's base
    /// address (if any).
    pub fn map(
        &mut self,
        vaddr: Address,
        paddr: Address,
    ) -> Result<Option<Address>, PageTableError> {
        if !vaddr.get().is_multiple_of(self.page_size)
            || !paddr.get().is_multiple_of(self.page_size)
        {
            return Err(PageTableError::Unaligned);
        }
        let vpn = self.page_number(vaddr);
        let ppn = self.page_number(paddr);
        Ok(self
            .entries
            .insert(vpn, ppn)
            .map(|old_ppn| Address::new(old_ppn * self.page_size)))
    }

    /// Remove the mapping for the page containing `vaddr`, returning its
    /// physical page's base address if it was mapped.
    pub fn unmap(&mut self, vaddr: Address) -> Option<Address> {
        let vpn = self.page_number(vaddr);
        self.entries
            .remove(&vpn)
            .map(|ppn| Address::new(ppn * self.page_size))
    }

    pub fn is_mapped(&self, vaddr: Address) -> bool {
        self.entries.contains_key(&self.page_number(vaddr))
    }

    /// Translate a single virtual address to its physical address.
    /// Page-faults (`Exception::PageFault`) if the containing page has no
    /// mapping.
    pub fn translate(&self, vaddr: Address) -> Result<Address, Exception> {
        let vpn = self.page_number(vaddr);
        let offset = vaddr.get() % self.page_size;
        self.entries
            .get(&vpn)
            .map(|&ppn| Address::new(ppn * self.page_size + offset))
            .ok_or(Exception::PageFault { address: vaddr })
    }

    /// Translate `[vaddr, vaddr+len)` (which must lie within one virtual
    /// page -- see the module-level "Documented scope" note) and check
    /// that the resulting physical range is permitted in `space`.
    pub fn translate_checked(
        &self,
        space: &AddressSpace,
        vaddr: Address,
        len: u64,
        requested: Permission,
    ) -> Result<Address, Exception> {
        if len > 0 {
            let last_byte = vaddr
                .get()
                .checked_add(len - 1)
                .ok_or(Exception::MisalignedAccess { address: vaddr })?;
            if self.page_number(vaddr) != self.page_number(Address::new(last_byte)) {
                return Err(Exception::MisalignedAccess { address: vaddr });
            }
        }
        let paddr = self.translate(vaddr)?;
        if len > 0 && !space.is_permitted(paddr, len, requested) {
            return Err(Exception::PermissionViolation { address: paddr });
        }
        Ok(paddr)
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_address_space::Region;
    use jxcl_memory::Memory;

    fn page(n: u64) -> Address {
        Address::new(n * DEFAULT_PAGE_SIZE)
    }

    #[test]
    fn map_then_translate_within_a_page() {
        let mut pt = PageTable::new();
        pt.map(page(0), page(5)).unwrap();
        let vaddr = Address::new(0x10);
        assert_eq!(
            pt.translate(vaddr).unwrap(),
            Address::new(5 * DEFAULT_PAGE_SIZE + 0x10)
        );
    }

    #[test]
    fn unmapped_access_page_faults() {
        let pt = PageTable::new();
        let err = pt.translate(Address::new(0x1000)).unwrap_err();
        assert_eq!(
            err,
            Exception::PageFault {
                address: Address::new(0x1000)
            }
        );
    }

    #[test]
    fn unmap_removes_the_mapping() {
        let mut pt = PageTable::new();
        pt.map(page(0), page(1)).unwrap();
        assert!(pt.is_mapped(page(0)));
        let old = pt.unmap(page(0)).unwrap();
        assert_eq!(old, page(1));
        assert!(!pt.is_mapped(page(0)));
        assert!(pt.translate(page(0)).is_err());
    }

    #[test]
    fn remapping_returns_the_previous_physical_page() {
        let mut pt = PageTable::new();
        pt.map(page(0), page(1)).unwrap();
        let old = pt.map(page(0), page(2)).unwrap();
        assert_eq!(old, Some(page(1)));
        assert_eq!(pt.translate(page(0)).unwrap(), page(2));
    }

    #[test]
    fn unaligned_map_is_rejected() {
        let mut pt = PageTable::new();
        assert_eq!(
            pt.map(Address::new(1), page(0)).unwrap_err(),
            PageTableError::Unaligned
        );
    }

    #[test]
    fn invalid_page_size_is_rejected() {
        assert_eq!(
            PageTable::with_page_size(3).unwrap_err(),
            PageTableError::InvalidPageSize
        );
    }

    #[test]
    fn translate_checked_page_faults_on_unmapped() {
        let pt = PageTable::new();
        let space = AddressSpace::new(Memory::new(DEFAULT_PAGE_SIZE));
        let err = pt
            .translate_checked(&space, Address::new(0), 4, Permission::READ_ONLY)
            .unwrap_err();
        assert_eq!(
            err,
            Exception::PageFault {
                address: Address::new(0)
            }
        );
    }

    #[test]
    fn translate_checked_denies_by_physical_permission() {
        let mut pt = PageTable::new();
        pt.map(page(0), page(0)).unwrap();
        let mut space = AddressSpace::new(Memory::new(DEFAULT_PAGE_SIZE));
        space
            .add_region(Region::new(
                "rodata",
                Address::new(0),
                DEFAULT_PAGE_SIZE,
                Permission::READ_ONLY,
            ))
            .unwrap();
        let err = pt
            .translate_checked(&space, Address::new(0), 4, Permission::READ_WRITE)
            .unwrap_err();
        assert_eq!(err, Exception::PermissionViolation { address: page(0) });
    }

    #[test]
    fn translate_checked_succeeds_when_mapped_and_permitted() {
        let mut pt = PageTable::new();
        pt.map(page(0), page(3)).unwrap();
        let mut space = AddressSpace::new(Memory::new(4 * DEFAULT_PAGE_SIZE));
        space
            .add_region(Region::new(
                "data",
                page(3),
                DEFAULT_PAGE_SIZE,
                Permission::READ_WRITE,
            ))
            .unwrap();
        let paddr = pt
            .translate_checked(&space, Address::new(0x10), 4, Permission::READ_WRITE)
            .unwrap();
        assert_eq!(paddr, Address::new(3 * DEFAULT_PAGE_SIZE + 0x10));
    }

    #[test]
    fn access_crossing_a_page_boundary_is_misaligned() {
        let mut pt = PageTable::new();
        pt.map(page(0), page(0)).unwrap();
        pt.map(page(1), page(1)).unwrap();
        let mut space = AddressSpace::new(Memory::new(2 * DEFAULT_PAGE_SIZE));
        space
            .add_region(Region::new(
                "data",
                Address::new(0),
                2 * DEFAULT_PAGE_SIZE,
                Permission::READ_WRITE,
            ))
            .unwrap();
        let vaddr = Address::new(DEFAULT_PAGE_SIZE - 2);
        let err = pt
            .translate_checked(&space, vaddr, 4, Permission::READ_ONLY)
            .unwrap_err();
        assert_eq!(err, Exception::MisalignedAccess { address: vaddr });
    }
}
