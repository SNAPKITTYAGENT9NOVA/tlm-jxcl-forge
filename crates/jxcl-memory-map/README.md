# jxcl-memory-map

The concrete default memory layout (where code/stack/heap/MMIO live) consumed by the loader and machine setup.

## Architecture

**Owns:** `MemoryMap`, the `DEFAULT_MEMORY_MAP` default layout constant.

**Category:** memory · **Source:** new. There is no single pre-expansion
"fixed memory map" to extract (`crates/jxcl/src/main.rs`'s `run`
subcommand sizes memory ad hoc, per program); see the module doc for the
documented, explicit fractional split (code 40% / data 20% / heap 30% /
mmio 2% / stack remainder) this crate uses instead, sized against
`jxcl-constants::MEMORY_SIZE`.

## Public API

- `MemoryMap` -- `sized(total_size)`, `regions()`, `install(&mut AddressSpace)`
- `MemoryRegionSpec` -- one named/sized/permissioned entry
- `DEFAULT_MEMORY_MAP` -- the map sized against `jxcl-constants::MEMORY_SIZE`

## Dependencies

Workspace crates:

- `jxcl-address-space`
- `jxcl-constants`
- `jxcl-types`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-memory-map`: unit tests that the five regions exactly
tile `total_size` with no gaps or overlaps, that the code region is
read+execute (not write), that the stack sits at the top of the address
space, that `install` populates a real `AddressSpace` without error, and
that `sized` scales correctly to an arbitrary total.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
