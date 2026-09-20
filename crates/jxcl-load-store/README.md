# jxcl-load-store

Typed, sign-extension-aware load/store helpers (u8/u16/u32/u64 and signed variants) between raw memory and the execution engine.

## Architecture

**Owns:** `load_u8/16/32/64`, `store_u8/16/32/64` and their signed
counterparts (`load_i8/16/32/64`, `store_i8/16/32/64`).

**Category:** memory · **Source:** new (generalizes
`crates/jxcl/src/execution.rs`'s direct `state.memory.read64`/`write64`
+ `try_mem!` fault-conversion pattern to every width and to signed
values, converting into `jxcl-exceptions::Exception`).

## Public API

`load_u8`, `load_u16`, `load_u32`, `load_u64`, `load_i8`, `load_i16`,
`load_i32`, `load_i64`, `store_u8`, `store_u16`, `store_u32`,
`store_u64`, `store_i8`, `store_i16`, `store_i32`, `store_i64` -- all
operating on a `jxcl-memory::Memory` at a `jxcl-types::Address`,
returning `Result<_, jxcl_exceptions::Exception>`.

## Dependencies

Workspace crates:

- `jxcl-memory`
- `jxcl-endian`
- `jxcl-exceptions`
- `jxcl-types`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-load-store`: unit tests for unsigned roundtrip at
every width, correct sign extension on signed loads, truncation on
signed stores, `MemoryFault` -> `Exception` conversion on out-of-bounds
and misaligned access, and a property-style roundtrip sweep over several
representative values (including width extremes) for both unsigned and
signed 64-bit paths.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
