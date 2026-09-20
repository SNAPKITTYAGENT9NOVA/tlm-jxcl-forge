# jxcl-flags

The flags register: zero/carry/overflow/negative bit semantics.

## Architecture

**Owns:** the `Flags` type (decoded Z/N/C/V view), `FlagEffect`
(which flags a given instruction is permitted to modify, spec §6), and
`apply_effect` (the masked merge of a freshly computed `Flags` into a
live flags word).

Extracted from `jxcl/src/isa/flags.rs`, keeping the exact bit layout
(`Z` bit 0, `N` bit 1, `C` bit 2, `V` bit 3) and masked-merge semantics,
and additionally exposing `Flags::from_word`/`to_word` over
`jxcl-types::Word` so callers that already work in that foundation type
don't have to unwrap it to a bare `u64` first.

**Category:** isa · **Source:** extraction:jxcl/src/isa/flags.rs

## Public API

- `Flags { z, n, c, v }` — `from_bits`, `to_bits`, `from_word`, `to_word`, `zn_of`
- `FlagEffect { z, n, c, v }` — `NONE`, `ZN`, `ZNC`, `ZNV`, `ZNCV_FULL`
- `apply_effect(current_bits, effect, computed) -> u64`
- `Z_BIT`, `N_BIT`, `C_BIT`, `V_BIT`

## Dependencies

Workspace crates:

- `jxcl-types`

External crates:

*(none)*

## Examples

```rust
use jxcl_flags::{apply_effect, FlagEffect, Flags};

let (z, n) = Flags::zn_of(0u64.wrapping_sub(1)); // 0xFFFF...FFFF
let computed = Flags { z, n, c: true, v: false };

// ADD affects Z, N, C, V; carry/overflow are computed by the ALU.
let bits = apply_effect(0, FlagEffect::ZNCV_FULL, computed);
assert_eq!(Flags::from_bits(bits), computed);
```

## Testing

`cargo test -p jxcl-flags` — 10 unit/property-style tests: per-flag
bit decoding, `from_bits`/`to_bits` round-tripping over all 16 possible
4-bit patterns, the `Word` round-trip, `zn_of` on zero/negative/positive
results, and `apply_effect` for `NONE`/`ZN`/`ZNV`/`ZNCV_FULL` masks
(confirming untouched bits are preserved, not cleared).

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
