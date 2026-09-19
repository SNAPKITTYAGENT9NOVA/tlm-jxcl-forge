# jxcl-encoding

Encodes an `Instruction` to its binary wire format.

## Architecture

**Owns:** `encode(Instruction) -> bytes` -- the only place instruction
bit layout is written.

Extracted from `jxcl/src/encoding/encoder.rs`. Structural legality (does
the given operand shape match what the opcode registry declares for its
mnemonic) is still checked here, via
`jxcl-instructions::format_matches_registry`; register-*range* legality
is not (that's `jxcl-instruction-validation`'s job -- see the module doc
for why, tied to this crate's frozen `docs/crates.toml` dependency
list). Byte writing goes through `jxcl-bytes::ByteCursorMut` and
`jxcl-endian` (via the cursor's `write_*` methods) instead of calling
`to_le_bytes()` inline, and every signed 32-bit displacement write is
cross-checked in debug builds against `jxcl-bitops::sign_extend` -- the
same primitive `jxcl-decoding` uses to read displacements back -- so an
encode/decode sign-extension disagreement would fail loudly rather than
silently breaking the round-trip invariant.

**Category:** isa · **Source:** extraction:jxcl/src/encoding/encoder.rs

## Public API

- `encode(instr: &DecodedInstruction, out: &mut Vec<u8>) -> Result<usize, EncodeError>`
- `encode_all(instrs: &[DecodedInstruction]) -> Result<Vec<u8>, EncodeError>`

## Dependencies

Workspace crates:

- `jxcl-instructions`
- `jxcl-bitops`
- `jxcl-endian`
- `jxcl-bytes`
- `jxcl-errors`

External crates:

*(none)*

## Examples

```rust
use jxcl_encoding::encode_all;
use jxcl_instructions::{Instruction, Mnemonic, Operands, Register};

let program = [
    Instruction::new(Mnemonic::Movi, Operands::RImm64 { rd: Register::new(1), imm: 1 }),
    Instruction::new(Mnemonic::Halt, Operands::None),
];
let bytes = encode_all(&program).unwrap();
assert_eq!(bytes.len(), 10 + 1);
```

## Testing

`cargo test -p jxcl-encoding` — 8 unit tests covering every operand
shape's exact byte layout (`None`, `RR`, `RImm64`, `BranchImm32`'s
negative-displacement little-endian encoding, `Cas`'s four-field
order), rejection of a mismatched operand shape (with a check that
nothing is written on failure), `encode_all` concatenation, and that
`encode` appends without disturbing bytes already in the output buffer.

## Status

`implemented`.
