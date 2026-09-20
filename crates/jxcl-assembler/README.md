# jxcl-assembler

The two-pass assembler: source text to an object file (plus a convenience path assembling and linking a single file straight to a binary, preserving the original single-step CLI workflow).

## Purpose

The assembler performs two standard deterministic passes over assembly source text:

1. **Layout pass** — walks every statement, computing final offsets for labels and constants by summing instruction and data widths from the opcode registry. Constants are evaluated immediately (they can only reference literals and earlier constants, never labels). Labels are recorded with their section-local offsets.

2. **Emission pass** — walks the statements again, resolving every label/constant reference against the now-complete symbol table and emitting real instruction bytes via `jxcl-encoding`.

The assembler produces a relocatable [`ObjectFile`] (code bytes, data bytes, section-local symbol table, and relocation records) rather than a finished binary. A second phase, [`assemble_to_binary`], calls [`jxcl_linker::link`] to resolve all symbols and apply relocations, producing the final [`BinaryContainer`].

## Public API

- [`assemble`]: `(source: &str) -> Result<ObjectFile, AssemblerError>` — Assemble source text into a relocatable object file.
- [`assemble_to_binary`]: `(source: &str) -> Result<BinaryContainer, AssemblerError>` — Assemble and link in one step (convenience for single-file workflows, preserving the original CLI's single-step behavior).

## Implementation Notes

### Label and Constant Resolution

The assembler maintains three symbol classes:

- **Code labels**: assigned section-local addresses in the code section during pass 1; used for direct references and branch targets.
- **Data labels**: assigned section-local addresses in the data section during pass 1; converted to absolute addresses during linking (offset by the total code size).
- **Constants**: evaluated immediately in pass 1; must not reference labels or be defined out of order. Used to provide compile-time values to immediates and displacements.

### Forward References and Relocations

The assembler uses a single relocation table (not separate immediate and branch tables) to handle forward references. Every label reference in an instruction operand is recorded as a relocation (Absolute64 for `MOVI`, PcRelative32 for branches), even if the label is defined later in the same section. The linker then applies these relocations with correct final addresses.

### Two-Function Workflow

- **Single-file case**: Call `assemble_to_binary` once; symbol resolution and relocation happen transparently via the linker.
- **Multi-file case** (future): Call `assemble` once per source file, collect all [`ObjectFile`]s, and pass them to `jxcl_linker::link`.

## Testing

The crate includes 16 tests:

- **Unit tests** (10): label resolution, duplicate detection, constant evaluation, mnemonic validation, operand arity checking, range validation, section switching, and empty programs.
- **Integration tests** (2): end-to-end assembly with forward branches and multi-instruction sequences.
- **Golden tests** (4): deterministic output on a multi-instruction program, code + data sections with relocations, and boundary cases (empty, comments).

Run with: `cargo test -p jxcl-assembler`

## Dependencies

Workspace crates:

- `jxcl-lexer` — tokenization
- `jxcl-parser` — AST parsing
- `jxcl-object` — relocatable object file format
- `jxcl-symbols` — symbol table management
- `jxcl-relocations` — relocation data types and application
- `jxcl-encoding` — instruction byte encoding
- `jxcl-instructions` — decoded instruction type
- `jxcl-linker` — linking pass 1 and 2
- `jxcl-binary` — final executable container format

External crates:

*(none)*

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
