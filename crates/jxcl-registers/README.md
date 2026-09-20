# jxcl-registers

The register file model: indices, names, widths.

## Architecture

**Owns:** the `Register` register-index alias (over
`jxcl-types::RegisterIndex`) and the `RegisterFile` type: 32
general-purpose registers plus the special `pc`/`sp`/`fp`/`flags`
registers (spec §3, §5).

Extracted from `jxcl/src/isa/registers.rs`, adapted to use
`jxcl-types::RegisterIndex` in place of a bare `u8` and
`jxcl-constants::{REGISTER_COUNT, R0_HARDWIRED_ZERO}` in place of
locally duplicated constants. Register-range errors are reported with a
small crate-local `RegisterOutOfRange` type rather than
`jxcl-errors::ExecutionFault`, since this crate (per `docs/crates.toml`)
depends only on `jxcl-types` and `jxcl-constants`.

**Category:** isa · **Source:** extraction:jxcl/src/isa/registers.rs

## Public API

- `Register` (= `jxcl_types::RegisterIndex`)
- `RegisterOutOfRange`
- `validate_register(id: Register) -> Result<(), RegisterOutOfRange>`
- `RegisterFile` — `new`, `read`, `write`, `general_registers`,
  `set_general_registers`, plus public `pc`/`sp`/`fp`/`flags` fields

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-constants`

External crates:

*(none)*

## Examples

```rust
use jxcl_registers::{Register, RegisterFile};

let mut rf = RegisterFile::new();
rf.write(Register::new(5), 42).unwrap();
assert_eq!(rf.read(Register::new(5)).unwrap(), 42);

// R0 is hardwired to zero: writes are discarded.
rf.write(Register::new(0), 0xdead_beef).unwrap();
assert_eq!(rf.read(Register::new(0)).unwrap(), 0);
```

## Testing

`cargo test -p jxcl-registers` — 9 unit tests: register-range
validation (in range / out of range), R0 hardwiring on both read and
write, general-purpose register round-tripping, `set_general_registers`
forcing R0 back to zero, special-register defaults, and the
`RegisterOutOfRange` `Display` impl.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
