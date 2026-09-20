# jxcl-disassembler

Disassembles a binary/object file back to readable assembly text, symbolizing addresses when debug info is present.

## Purpose

Converts raw JXCL machine code back into canonical assembly text. The disassembler produces a canonical form that avoids inventing label names: every address operand (branch targets, entry points) is printed as the absolute resolved address itself in decimal. This ensures the output can be reassembled byte-for-byte identically (round-trip invariant) without needing a symbol table.

## Public API

**`disassemble(code: &[u8], data: &[u8], entry_point: u64) -> Result<String, DecodeError>`**

Disassembles a code section and optional data section into canonical JXCL assembly text. If `entry_point` is non-zero, it is emitted as a `.entry` directive. The data section, if present, is rendered as `.byte` directives in 16-byte chunks for canonical width-agnostic output.

**`render_instruction(mnemonic: Mnemonic, operands: Operands, offset: u64, length: u64) -> String`**

Renders a single decoded instruction to its canonical textual form. Used by the disassembler and also useful for debuggers/trace modes that render instructions interleaved with register state.

## Implementation Notes

- Extracted from `jxcl/src/disassembler.rs` with adaptation for the split dependency crates.
- Decoding is handled by `jxcl-decoding::decode_all`, which returns a vector of (offset, DecodedInstruction) tuples.
- Branch target calculation is inlined: `target = pc_after_fetch + displacement` where `pc_after_fetch = offset + instruction_length`.
- Memory operands with displacements are formatted with optional `+` or `-` indicators (e.g., `[R2]`, `[R2+8]`, `[R2-4]`).
- The data section uses 16-byte-per-line chunking to keep the output width-agnostic.
- No unsafe code; `#![forbid(unsafe_code)]` is enforced.

## Testing

40 comprehensive tests covering:
- Unit tests for each instruction operand format (None, R, RR, RRR, RImm64, RMem, MemR, Cas, BranchImm32, Imm16)
- Positive and negative displacement rendering
- Empty code/data sections
- Entry point directive emission
- Data section formatting (single byte, multiple chunks)
- Error handling (truncated instructions, invalid opcodes)
- Three golden-vector integration tests (simple arithmetic, branching, memory operations)

## Dependencies

Workspace crates:

- `jxcl-decoding` — instruction decoder
- `jxcl-instructions` — instruction/operand types and mnemonic registry
- `jxcl-symbols` — (re-exported, not directly used in this implementation)
- `jxcl-debug-info` — (re-exported, not directly used in this implementation)

External crates:

*(none)*

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
