# jxcl-address-space

Named memory regions (code/data/stack) with permission flags layered over raw storage.

## Architecture

**Owns:** `AddressSpace` and `Region` (base, len, permissions).

**Category:** memory · **Source:** new (generalizes
`crates/jxcl/src/memory.rs`'s hard-coded code/data two-region permission
split into an arbitrary list of named regions, each with its own
`Permission`, layered over `jxcl-memory`'s raw storage).

## Public API

- `AddressSpace::new`, `add_region`, `is_permitted`, `region_containing`,
  `memory`/`memory_mut`, `regions`
- `Region::new`, `end`, `contains_range`
- `Permission` -- `read`/`write`/`execute` bits, `NONE`/`READ_ONLY`/
  `READ_WRITE`/`READ_EXECUTE` constants, `allows`

Unmapped addresses, and accesses that straddle two regions, are denied
by default (deny-by-default, matching a real MMU rather than
`crates/jxcl/src/memory.rs`'s "everything not code is read/write").

## Dependencies

Workspace crates:

- `jxcl-memory`
- `jxcl-types`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-address-space`: unit + boundary tests for in-bounds
vs. out-of-bounds region registration, overlap rejection (including the
adjacent-is-fine case), permission checks against requested bits,
default-deny on unmapped addresses, and denial of an access that
straddles two adjacent regions.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
