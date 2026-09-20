# jxcl-instructions

The `Instruction` type tying an opcode to its operands -- the canonical
in-memory instruction representation.

## Architecture

**Owns:** `DecodedInstruction` (aliased as `Instruction`): a mnemonic
paired with its decoded operands and the pre-computed encoded length.
This is both the encoder's input and the decoder's output.

Extracted from `jxcl/src/isa/instruction.rs`, unchanged in shape. This
crate is also the mechanical bridge between `jxcl-opcodes` and
`jxcl-operands` for the crates below it: `docs/crates.toml` gives
`jxcl-encoding`/`jxcl-decoding` a dependency list of
`jxcl-instructions`, `jxcl-bitops`, `jxcl-endian`, `jxcl-bytes`,
`jxcl-errors` -- deliberately not `jxcl-opcodes`/`jxcl-operands`
directly -- so this crate re-exports `Mnemonic` (from `jxcl-opcodes`)
and `Format`/`Operands`/`Register` (from `jxcl-operands`), and provides
the one piece of glue only possible here: `jxcl-opcodes::Format` (the
registry's per-opcode wire-format tag) and `jxcl-operands::Format` (an
operand set's actual shape) are two structurally identical but distinct
Rust types -- `jxcl-opcodes` cannot depend on `jxcl-operands` without
creating a cycle (see `jxcl-opcodes`'s module doc) -- so
`format_matches_registry`/`instruction_shape_for_opcode` are the
mechanical conversion between them.

**Category:** isa · **Source:** extraction:jxcl/src/isa/instruction.rs

## Public API

- `DecodedInstruction` (= `Instruction`) `{ mnemonic, operands, length }`
  — `DecodedInstruction::new(mnemonic, operands) -> Self`
- Re-exported: `Mnemonic`, `Format`, `Operands`, `Register`
- `opcode_byte(mnemonic) -> u8`
- `instruction_shape_for_opcode(opcode) -> Option<(Mnemonic, Format)>`
- `format_matches_registry(&DecodedInstruction) -> bool`

## Dependencies

Workspace crates:

- `jxcl-opcodes`
- `jxcl-operands`

Dev-dependencies (tests only):

- `jxcl-registers` (to construct `Register` values for test operands)

External crates:

*(none)*

## Examples

```rust
use jxcl_instructions::{format_matches_registry, Instruction, Mnemonic, Operands, Register};

let instr = Instruction::new(
    Mnemonic::Movi,
    Operands::RImm64 { rd: Register::new(0), imm: 42 },
);
assert_eq!(instr.length, 10); // opcode + rd + imm64
assert!(format_matches_registry(&instr));

// Wrong operand shape for ADD (registered as RR) is caught mechanically:
let bad = Instruction::new(Mnemonic::Add, Operands::R { rd: Register::new(1) });
assert!(!format_matches_registry(&bad));
```

## Testing

`cargo test -p jxcl-instructions` — 10 unit tests: `length` derivation,
`length` matching `operands.format().len()` across all ten operand
shapes, the `Instruction` alias, field pass-through, `opcode_byte`
against known registry entries, `instruction_shape_for_opcode`
round-tripping (including the unassigned-opcode `None` case),
`format_matches_registry` accepting/rejecting shapes, and an exhaustive
round-trip of the opcode/operand `Format` bridge over every registered
instruction.

## Status

`implemented`.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
