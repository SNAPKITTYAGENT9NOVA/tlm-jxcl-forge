# jxcl-cache-model

A direct-mapped cache simulation layered over memory, for profiling hit/miss behavior.

## Architecture

**Owns:** `CacheModel` and its hit/miss accounting.

**Category:** memory · **Source:** new. A simulation, not a functional
cache: it never changes what a load/store returns, it only tracks
which line an address maps to and whether that line currently holds a
matching tag, for `jxcl-profiler` (a later batch) to aggregate.

**Deviation from `docs/crates.toml`:** `jxcl-types` is added as a direct
dependency, alongside the registry's listed `jxcl-memory` and
`jxcl-address-space`. Both of those crates' real public APIs address
memory with `jxcl-types::Address`; naming that same type in this
crate's public API needs `jxcl-types` declared directly.

## Public API

- `CacheModel::new(line_size, num_lines)` (both must be powers of two)
- `access(addr)` / `access_range(addr, len)` -> `CacheEvent::Hit`/`Miss`
- `stats()` -> `CacheStats { hits, misses }`, with `hit_rate()`
- `flush()` -- invalidate all lines without resetting statistics

## Dependencies

Workspace crates:

- `jxcl-memory`
- `jxcl-address-space`
- `jxcl-types`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-cache-model`: rejection of non-power-of-two
parameters, a known-pattern test (first access misses, repeat access to
the same line hits), a known-pattern conflict-miss test (two addresses
that alias the same line but different tags thrash), hit-rate
computation (including the zero-accesses case), multi-line
`access_range` accounting, and flush-without-reset-of-stats.

## Status

`implemented`.
