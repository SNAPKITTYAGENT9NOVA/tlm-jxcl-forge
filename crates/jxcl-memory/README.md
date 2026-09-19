# jxcl-memory

Raw byte-addressable memory storage and bounds-checked byte-level access.

## Architecture

**Owns:** The `Memory` struct -- the only place raw bytes are stored.

**Category:** memory · **Source:** new (grounded in the bounds/alignment
rules of `crates/jxcl/src/memory.rs`, minus that file's code/data
permission split, which is `jxcl-address-space`'s invariant, layered on
top of this crate rather than duplicated here).

## Public API

- `Memory::new(size)` / `Memory::from_bytes(bytes)`
- `read8/16/32/64` and `write8/16/32/64`, addressed by `jxcl_types::Address`,
  little-endian (via `jxcl-endian`), bounds- and alignment-checked
- `read_bytes`/`write_bytes` for alignment-free raw byte-range access
  (used by instruction fetch and the loader)
- `snapshot`/`restore` for full-buffer save/restore

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-errors`
- `jxcl-endian`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-memory`: unit tests covering read/write roundtrip,
little-endian byte order, alignment faults on misaligned 16/32/64-bit
access, out-of-bounds detection (including address-overflow avoiding a
panic), alignment-free byte-range access, and snapshot/restore.

## Status

`implemented`.
