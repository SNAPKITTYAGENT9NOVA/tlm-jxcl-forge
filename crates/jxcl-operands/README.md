# jxcl-operands

The operand/addressing-mode model (register-direct, immediate,
memory-indirect, etc.).

## Architecture

**Owns:** the instruction wire-format tag (`Format`, aliased as
`AddressingMode`) and the decoded operand values (`Operands`, aliased
as `Operand`) -- each `Operands` variant corresponds 1:1 with a
`Format` variant, so an instruction's operands can never mismatch its
declared format.

Extracted from `jxcl/src/isa/operand.rs`, with register fields typed as
`jxcl-registers::Register` in place of the pre-expansion bare
`RegId = u8`. Adds `Operands::registers()`, a new helper (not present
pre-expansion) that returns every register field an operand set
references, in encoding order -- used by `jxcl-instruction-validation`
so it doesn't have to re-derive the per-format field layout.

**Category:** isa · **Source:** extraction:jxcl/src/isa/operand.rs

## Public API

- `Format` (= `AddressingMode`) — `None`, `R`, `RR`, `RRR`, `RImm64`,
  `RMem`, `MemR`, `Cas`, `BranchImm32`, `Imm16`; `Format::len()`
- `Operands` (= `Operand`) — one variant per `Format`; `Operands::format()`,
  `Operands::registers()`

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-registers`

External crates:

*(none)*

## Examples

```rust
use jxcl_operands::{Format, Operands};
use jxcl_registers::Register;

let ops = Operands::RR {
    rd: Register::new(1),
    rs: Register::new(2),
};
assert_eq!(ops.format(), Format::RR);
assert_eq!(ops.format().len(), 3); // opcode + rd + rs
assert_eq!(ops.registers(), vec![Register::new(1), Register::new(2)]);
```

## Testing

`cargo test -p jxcl-operands` — 5 unit tests: `Format::len()` against
every entry of `docs/ISA_SPEC.md`'s encoding table, `Operands::format()`
consistency across representative variants, `registers()` extraction
(including the CAS format's four-field layout) and its emptiness for
immediate/branch-only forms, and that the `AddressingMode`/`Operand`
aliases really are the primary types.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
