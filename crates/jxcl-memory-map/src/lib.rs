//! The concrete default memory layout (where code/data/heap/stack/MMIO
//! live) consumed by the loader and machine setup.
//!
//! ## Documented sizing assumption
//!
//! `crates/jxcl/src/main.rs`'s pre-expansion `run` subcommand sizes a
//! program's memory ad hoc, per-run, as `code_size + data_size +
//! headroom` (`jxcl-constants::MEMORY_SIZE`, aliased from that literal
//! `1 << 20` headroom constant -- see that crate's doc comment). There is
//! no single pre-expansion "here is the fixed memory map" layout to
//! extract, so this crate makes one up, sized against
//! `jxcl-constants::MEMORY_SIZE` (1 MiB), and documents the split as an
//! explicit, arbitrary-but-reasonable policy rather than pretending it
//! comes from the spec:
//!
//! | region | fraction of `MEMORY_SIZE` | permission    |
//! |--------|---------------------------|---------------|
//! | code   | 40%                       | read+execute  |
//! | data   | 20%                       | read+write    |
//! | heap   | 30%                       | read+write    |
//! | mmio   | 2%                        | read+write    |
//! | stack  | remainder (~8%), at the top of the address space, since the ISA's stack grows downward (`jxcl-constants::STACK_GROWS_DOWNWARD`) | read+write |
#![forbid(unsafe_code)]

use jxcl_address_space::{AddressSpace, AddressSpaceError, Permission, Region};
use jxcl_constants::MEMORY_SIZE;
use jxcl_types::Address;

/// One named, sized, permissioned entry in a [`MemoryMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegionSpec {
    pub name: &'static str,
    pub base: u64,
    pub len: u64,
    pub permission: Permission,
}

/// The full default memory layout: five named regions covering the
/// entire address space with no gaps and no overlaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMap {
    pub total_size: u64,
    pub code: MemoryRegionSpec,
    pub data: MemoryRegionSpec,
    pub heap: MemoryRegionSpec,
    pub mmio: MemoryRegionSpec,
    pub stack: MemoryRegionSpec,
}

impl MemoryMap {
    /// Build a memory map sized against `total_size`, using this crate's
    /// documented fractional split.
    pub const fn sized(total_size: u64) -> Self {
        let code_len = total_size * 40 / 100;
        let data_len = total_size * 20 / 100;
        let heap_len = total_size * 30 / 100;
        let mmio_len = total_size * 2 / 100;
        // Remainder (not exactly 8% due to integer division) goes to the
        // stack, so the five regions always sum to exactly `total_size`.
        let stack_len = total_size - code_len - data_len - heap_len - mmio_len;

        let code_base = 0;
        let data_base = code_base + code_len;
        let heap_base = data_base + data_len;
        let mmio_base = heap_base + heap_len;
        let stack_base = mmio_base + mmio_len;

        MemoryMap {
            total_size,
            code: MemoryRegionSpec {
                name: "code",
                base: code_base,
                len: code_len,
                permission: Permission::READ_EXECUTE,
            },
            data: MemoryRegionSpec {
                name: "data",
                base: data_base,
                len: data_len,
                permission: Permission::READ_WRITE,
            },
            heap: MemoryRegionSpec {
                name: "heap",
                base: heap_base,
                len: heap_len,
                permission: Permission::READ_WRITE,
            },
            mmio: MemoryRegionSpec {
                name: "mmio",
                base: mmio_base,
                len: mmio_len,
                permission: Permission::READ_WRITE,
            },
            stack: MemoryRegionSpec {
                name: "stack",
                base: stack_base,
                len: stack_len,
                permission: Permission::READ_WRITE,
            },
        }
    }

    /// All five regions, in address order.
    pub const fn regions(&self) -> [MemoryRegionSpec; 5] {
        [self.code, self.data, self.heap, self.mmio, self.stack]
    }

    /// Register every region of this map into `space` (whose backing
    /// memory must be at least `total_size` bytes).
    pub fn install(&self, space: &mut AddressSpace) -> Result<(), AddressSpaceError> {
        for r in self.regions() {
            space.add_region(Region::new(
                r.name,
                Address::new(r.base),
                r.len,
                r.permission,
            ))?;
        }
        Ok(())
    }
}

/// The default memory map, sized against `jxcl-constants::MEMORY_SIZE`.
pub const DEFAULT_MEMORY_MAP: MemoryMap = MemoryMap::sized(MEMORY_SIZE as u64);

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_memory::Memory;

    #[test]
    fn regions_exactly_cover_total_size_with_no_gaps_or_overlaps() {
        let map = DEFAULT_MEMORY_MAP;
        let regions = map.regions();
        let mut expected_base = 0u64;
        for r in regions {
            assert_eq!(
                r.base, expected_base,
                "region {} has a gap before it",
                r.name
            );
            expected_base += r.len;
        }
        assert_eq!(expected_base, map.total_size);
    }

    #[test]
    fn code_region_is_read_execute_not_write() {
        let map = DEFAULT_MEMORY_MAP;
        assert!(map.code.permission.read);
        assert!(map.code.permission.execute);
        assert!(!map.code.permission.write);
    }

    #[test]
    fn stack_sits_at_the_top_of_the_address_space() {
        let map = DEFAULT_MEMORY_MAP;
        assert_eq!(map.stack.base + map.stack.len, map.total_size);
    }

    #[test]
    fn install_populates_an_address_space_without_error() {
        let map = DEFAULT_MEMORY_MAP;
        let mut space = AddressSpace::new(Memory::new(map.total_size));
        map.install(&mut space).unwrap();
        assert_eq!(space.regions().len(), 5);
    }

    #[test]
    fn sized_scales_to_an_arbitrary_total() {
        let map = MemoryMap::sized(1000);
        assert_eq!(map.regions().iter().map(|r| r.len).sum::<u64>(), 1000);
    }

    #[test]
    fn default_memory_map_matches_jxcl_constants_memory_size() {
        assert_eq!(DEFAULT_MEMORY_MAP.total_size, MEMORY_SIZE as u64);
    }
}
