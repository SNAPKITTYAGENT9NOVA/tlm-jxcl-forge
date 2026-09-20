# jxcl-stack

Typed stack-pointer push/pop with overflow/underflow detection for call/return semantics.

## Architecture

**Owns:** `Stack::push_u64`/`pop_u64` and their bounds checks.

**Category:** memory · **Source:** new. Mirrors
`crates/jxcl/src/execution.rs`'s PUSH/POP/CALL/RET bounds checks
(spec §15: SP decrements before a write, increments after a read) but
generalized to an explicit `[low, high)` stack region and to the two
distinct fault kinds `jxcl-exceptions::Exception` already has
(`StackOverflow`/`StackUnderflow`) instead of one generic `StackFault`.
`RegisterFile::sp` (the real, already-implemented `jxcl-registers` type)
remains the live stack pointer; `Stack` only carries the region bounds.

**Deviation from `docs/crates.toml`:** `jxcl-memory` and `jxcl-types`
are added as direct dependencies alongside the registry's listed
`jxcl-load-store`, `jxcl-exceptions`, `jxcl-registers` -- needed to name
`Memory` and `Address` in this crate's own public API.

## Public API

- `Stack::new(low, high)`
- `push_u64(&mut RegisterFile, &mut Memory, value)`
- `pop_u64(&mut RegisterFile, &Memory)`
- `depth`, `is_empty`, `low`, `high`

## Dependencies

Workspace crates:

- `jxcl-load-store`
- `jxcl-exceptions`
- `jxcl-registers`
- `jxcl-memory`
- `jxcl-types`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-stack`: push/pop roundtrip, LIFO ordering across two
pushes, overflow at the low bound (and that `sp` doesn't move on a
failed push), underflow on an empty stack (and that `sp` doesn't move on
a failed pop), and `depth`/`is_empty` tracking.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
