//! Loads a linked binary container into a memory image, ready to run.
//!
//! New functionality: the pre-expansion `jxcl` crate had no separate
//! "loader" step -- `jxcl/src/main.rs`'s `run` subcommand inlined this
//! exact sequence (size a machine's memory as `code_size + data_size +
//! headroom`, build a `Memory::with_code_region`, `loader_write` the code
//! and data sections in) directly into the CLI. This crate extracts that
//! sequence into a reusable `load` so any embedder (`jxcl-simulator`, the
//! debugger, `jxcl-cli`) gets it for free.
//!
//! ## Deviation from `docs/crates.toml`
//!
//! The registry's `public_api` is `load(BinaryContainer) -> (Memory,
//! entry_point)`, and lists `jxcl-memory`/`jxcl-memory-map` as
//! dependencies. Two things changed since that entry was written:
//!
//! 1. **Permissions.** In the post-expansion split, `jxcl-memory` is
//!    *only* bounds/alignment-checked raw storage -- the code/data
//!    permission split the pre-expansion `Memory::with_code_region` gave
//!    every loaded program now belongs to `jxcl-address-space`
//!    (`AddressSpace`/`Region`/`Permission`, see that crate's own doc
//!    comment). Returning a bare `jxcl_memory::Memory` from `load` would
//!    silently drop that protection (code no longer read-only+execute,
//!    data no longer denied to instruction fetch), which is exactly the
//!    real invariant the pre-expansion loader enforced. So `load` returns
//!    an [`AddressSpace`] instead, and this crate additionally depends on
//!    `jxcl-address-space` (and `jxcl-types`, for `Address`) to build it.
//! 2. **Layout.** `jxcl-memory-map::MemoryMap::sized` lays out a *fixed*
//!    generic layout (40% code / 20% data / 30% heap / 2% mmio /
//!    remainder stack) for a from-scratch machine setup with no program
//!    loaded yet. A `BinaryContainer`'s own format (`jxcl-binary`'s
//!    module docs) already fixes code at address `0` and data
//!    immediately at `code_size` -- a *specific* binary's section sizes,
//!    not a percentage split. Forcing a specific binary's code/data into
//!    `MemoryMap`'s generic percentage regions would either misplace data
//!    (leaving a gap between the end of a small program's code and the
//!    fixed 40% boundary, contradicting `code_size` addressing that every
//!    branch/entry-point calculation in the toolchain assumes) or reject
//!    an oversized program outright. So `load` builds its own
//!    three-region space (`code` = exactly `code_size` bytes, RX;
//!    `data` = exactly `data_size` bytes immediately after, RW;
//!    `headroom` = the remainder, RW, for the stack and heap) sized
//!    against `jxcl_constants::MEMORY_SIZE` -- the exact same headroom
//!    constant `jxcl-memory-map` is itself sized against, and the exact
//!    quantity the pre-expansion `main.rs` added -- rather than reusing
//!    `MemoryMap::sized`'s fixed percentages. `jxcl-memory-map` remains a
//!    real dependency for documentation/consistency purposes (a reader
//!    comparing this crate's layout against `DEFAULT_MEMORY_MAP`'s should
//!    see the same headroom constant governing both), even though its
//!    `MemoryMap` type itself isn't the layout `load` produces.
#![forbid(unsafe_code)]

use std::fmt;

use jxcl_address_space::{AddressSpace, AddressSpaceError, Permission, Region};
use jxcl_binary::BinaryContainer;
use jxcl_constants::MEMORY_SIZE;
use jxcl_errors::MemoryFault;
use jxcl_memory::Memory;
use jxcl_types::Address;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderErrorKind {
    /// The combined code + data + headroom size overflows `u64`.
    SizeOverflow,
    /// A region could not be registered (should not happen for any
    /// binary this crate builds regions for internally, but surfaced
    /// rather than panicking).
    RegionSetupFailed,
    /// Writing the code or data section into memory failed (should not
    /// happen given the regions are sized to exactly fit, but surfaced
    /// rather than panicking).
    WriteFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderError {
    pub kind: LoaderErrorKind,
    pub reason: String,
}

impl fmt::Display for LoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "loader error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for LoaderError {}

fn err(kind: LoaderErrorKind, reason: impl Into<String>) -> LoaderError {
    LoaderError {
        kind,
        reason: reason.into(),
    }
}

impl From<AddressSpaceError> for LoaderError {
    fn from(e: AddressSpaceError) -> Self {
        err(LoaderErrorKind::RegionSetupFailed, e.to_string())
    }
}

impl From<MemoryFault> for LoaderError {
    fn from(e: MemoryFault) -> Self {
        err(LoaderErrorKind::WriteFailed, e.to_string())
    }
}

/// Load a linked [`BinaryContainer`] into a fresh, permissioned
/// [`AddressSpace`], ready to run: code at address `0` (read + execute,
/// not writable), data immediately following at `code_size` (read +
/// write, not executable), and a headroom region beyond that (read +
/// write, for the stack and heap) sized per
/// `jxcl_constants::MEMORY_SIZE`. Returns the address space together
/// with the program's entry point (an absolute address -- since code
/// always starts at `0`, this is exactly `container.entry_point`).
pub fn load(container: &BinaryContainer) -> Result<(AddressSpace, u64), LoaderError> {
    let code_len = container.code.len() as u64;
    let data_len = container.data.len() as u64;
    let headroom = MEMORY_SIZE as u64;

    let total_size = code_len
        .checked_add(data_len)
        .and_then(|v| v.checked_add(headroom))
        .ok_or_else(|| {
            err(
                LoaderErrorKind::SizeOverflow,
                "code_size + data_size + headroom overflows u64",
            )
        })?;

    let memory = Memory::new(total_size);
    let mut space = AddressSpace::new(memory);

    space.add_region(Region::new(
        "code",
        Address::new(0),
        code_len,
        Permission::READ_EXECUTE,
    ))?;
    space.add_region(Region::new(
        "data",
        Address::new(code_len),
        data_len,
        Permission::READ_WRITE,
    ))?;
    let headroom_base = code_len + data_len;
    let headroom_len = total_size - headroom_base;
    if headroom_len > 0 {
        space.add_region(Region::new(
            "headroom",
            Address::new(headroom_base),
            headroom_len,
            Permission::READ_WRITE,
        ))?;
    }

    space
        .memory_mut()
        .write_bytes(Address::new(0), &container.code)?;
    space
        .memory_mut()
        .write_bytes(Address::new(code_len), &container.data)?;

    Ok((space, container.entry_point))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_code_and_data_at_the_documented_addresses() {
        let container = BinaryContainer::new(0, vec![0x60, 0x60, 0x60], vec![9, 9]);
        let (space, entry) = load(&container).unwrap();
        assert_eq!(entry, 0);
        assert_eq!(
            space.memory().read_bytes(Address::new(0), 3).unwrap(),
            &[0x60, 0x60, 0x60]
        );
        assert_eq!(
            space.memory().read_bytes(Address::new(3), 2).unwrap(),
            &[9, 9]
        );
    }

    #[test]
    fn entry_point_is_passed_through_unchanged() {
        let container = BinaryContainer::new(4, vec![0x60; 8], vec![]);
        let (_, entry) = load(&container).unwrap();
        assert_eq!(entry, 4);
    }

    #[test]
    fn code_region_is_read_execute_not_write() {
        let container = BinaryContainer::new(0, vec![0x60], vec![1, 2]);
        let (space, _) = load(&container).unwrap();
        assert!(space.is_permitted(Address::new(0), 1, Permission::READ_EXECUTE));
        assert!(!space.is_permitted(Address::new(0), 1, Permission::READ_WRITE));
    }

    #[test]
    fn data_region_is_read_write_not_execute() {
        let container = BinaryContainer::new(0, vec![0x60], vec![1, 2]);
        let (space, _) = load(&container).unwrap();
        assert!(space.is_permitted(Address::new(1), 1, Permission::READ_WRITE));
        assert!(!space.is_permitted(Address::new(1), 1, Permission::new(false, false, true)));
    }

    #[test]
    fn headroom_region_covers_the_remainder_and_is_read_write() {
        let container = BinaryContainer::new(0, vec![0x60], vec![1, 2]);
        let (space, _) = load(&container).unwrap();
        assert_eq!(space.regions().len(), 3);
        let headroom = space
            .regions()
            .iter()
            .find(|r| r.name == "headroom")
            .unwrap();
        assert_eq!(headroom.base, Address::new(3));
        assert_eq!(headroom.len, MEMORY_SIZE as u64);
        assert!(space.is_permitted(Address::new(3), 1, Permission::READ_WRITE));
    }

    #[test]
    fn empty_binary_still_loads_with_only_headroom_reachable() {
        let container = BinaryContainer::new(0, vec![], vec![]);
        let (space, entry) = load(&container).unwrap();
        assert_eq!(entry, 0);
        // Both code and data regions are legitimately zero-length; only
        // the headroom region has any bytes.
        assert_eq!(space.regions().len(), 3);
        assert!(space.memory().read_bytes(Address::new(0), 0).is_ok());
    }

    #[test]
    fn total_memory_size_is_code_plus_data_plus_headroom() {
        let code = vec![0x60; 10];
        let data = vec![1u8; 5];
        let container = BinaryContainer::new(0, code, data);
        let (space, _) = load(&container).unwrap();
        assert_eq!(space.memory().len(), 10 + 5 + MEMORY_SIZE as u64);
    }
}
