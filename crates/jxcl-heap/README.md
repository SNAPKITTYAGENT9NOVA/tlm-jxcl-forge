# jxcl-heap

A simple bump allocator operating within an address space, for programs needing dynamic memory.

## Architecture

**Owns:** `Heap::alloc`/`free`.

**Category:** memory · **Source:** new.

**Documented minimal scope:** this is a bump (watermark) allocator.
`alloc` only ever moves the watermark forward; `free` is an intentional
no-op. The base ISA has no architectural notion of freeing memory (that
would be a userspace/runtime convention layered on top), so a real
free-list is out of scope here and left to a possible future crate that
can own that separate invariant without breaking this one's API.

**Deviation from `docs/crates.toml`:** `jxcl-types` is added as a direct
dependency (see `jxcl-cache-model`/`jxcl-stack`'s READMEs for the same
reasoning -- naming `Address` in this crate's own API requires it).

## Public API

- `Heap::new(base, len)` / `Heap::from_region(&AddressSpace, name)`
- `alloc(&mut AddressSpace, size, align) -> Result<Address, HeapError>`
  -- zero-initializes the returned range, permission-checked against
  the `AddressSpace`
- `free(addr)` -- documented no-op
- `used`, `remaining`, `base`, `limit`

## Dependencies

Workspace crates:

- `jxcl-address-space`
- `jxcl-load-store`
- `jxcl-exceptions`
- `jxcl-types`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-heap`: sequential allocations never overlap,
allocations respect the requested alignment, out-of-memory once the
heap is exhausted, rejection of a non-power-of-two alignment,
permission-denied when the backing region isn't writable, `free`
provably not reclaiming space, `from_region` lookup, and that allocated
bytes come back zeroed even over previously-dirty memory.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
