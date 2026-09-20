# jxcl-cli

The `jxcl` command-line interface: asm/disasm/run/inspect/validate subcommands.

## Purpose

Provides a complete command-line interface to the JXCL ISA toolchain, allowing users to:
- Assemble JXCL assembly source files to binary
- Disassemble binary files to readable assembly
- Validate binary files for structural correctness
- Inspect binary metadata (header, section sizes, instruction count)
- Run binary programs with optional execution limits

## Subcommand Reference

### `jxcl asm <input.jxcl> -o <output.jxc>`

Assembles JXCL assembly source to a binary executable.

**Arguments:**
- `<input.jxcl>`: Path to the assembly source file
- `-o <output.jxc>`, `--output <output.jxc>`: Path where the binary will be written

**Exit codes:**
- `0` — success
- `1` — usage error (missing or malformed arguments)
- `2` — I/O error reading input or writing output
- `3` — assembler error (syntax, undefined labels, etc.)

**Example:**
```bash
jxcl asm program.jxcl -o program.jxc
```

### `jxcl disasm <program.jxc>`

Disassembles a binary executable to its canonical assembly form and prints to stdout.

**Arguments:**
- `<program.jxc>`: Path to the binary file

**Exit codes:**
- `0` — success
- `1` — usage error (missing argument)
- `2` — I/O error reading file
- `3` — binary format or decode error

**Example:**
```bash
jxcl disasm program.jxc
```

### `jxcl validate <program.jxc>`

Validates a binary executable for structural correctness without executing it.
Decodes all instructions and checks branch targets. Prints instruction count
on success.

**Arguments:**
- `<program.jxc>`: Path to the binary file

**Exit codes:**
- `0` — success
- `1` — usage error (missing argument)
- `2` — I/O error reading file
- `3` — binary format or validation error

**Example:**
```bash
jxcl validate program.jxc
```

### `jxcl inspect <program.jxc>`

Displays metadata about a binary executable: entry point, code/data sizes,
and decoded instruction count.

**Arguments:**
- `<program.jxc>`: Path to the binary file

**Exit codes:**
- `0` — success
- `1` — usage error (missing argument)
- `2` — I/O error reading file
- `3` — binary format error

**Example:**
```bash
jxcl inspect program.jxc
```

### `jxcl run <program.jxc> [--limit N]`

Executes a binary program to completion (halt or fault) or until the execution
limit is reached. Prints final register state and cycle count on success.

**Arguments:**
- `<program.jxc>`: Path to the binary file
- `--limit N`: Maximum instruction steps to execute (default: 10,000,000)

**Exit codes:**
- `0` — success (program halted normally)
- `1` — usage error (missing or malformed arguments)
- `2` — I/O error reading file
- `3` — binary format or validation error
- `4` — program faulted or execution limit exceeded

**Example:**
```bash
jxcl run program.jxc
jxcl run program.jxc --limit 100000
```

## Implementation Notes

- No argument-parsing framework is used (per spec §40), employing instead a
  hand-rolled parser covering five subcommands with minimal flags deterministically.
- The CLI delegates to the split `jxcl-*` workspace crates:
  - **Assembly**: `jxcl-assembler::assemble_to_binary`
  - **Disassembly**: `jxcl-disassembler::disassemble`
  - **Binary parsing**: `jxcl-binary::BinaryContainer` and `jxcl-decoding::decode_all`
  - **Execution**: `jxcl-simulator::Simulator` for fast single-step or bulk execution
  - **Constants**: `jxcl-constants::DEFAULT_EXECUTION_LIMIT`
- Binary files follow the JXCL binary format (spec §27): a fixed-size header
  followed by code and data sections.
- All subcommands except `asm` read a serialized binary container; `asm` produces one.
- The `run` subcommand validates before executing to satisfy spec §29 (validation
  is performed once, before any instruction executes).

## Testing

**Integration tests** (5 per subcommand):
- `test_asm_basic`: assembles a simple program
- `test_asm_missing_output`: validates usage errors
- `test_disasm_basic`: assembles then disassembles, checking output contains mnemonics
- `test_disasm_missing_file`: validates I/O error handling
- `test_validate_basic`: assembles then validates, checking instruction count
- `test_validate_missing_file`: validates I/O error handling
- `test_inspect_basic`: assembles then inspects, checking header fields
- `test_inspect_missing_file`: validates I/O error handling
- `test_run_basic`: assembles then runs a simple arithmetic program
- `test_run_with_limit`: runs with a custom execution limit
- `test_run_missing_file`: validates I/O error handling
- `test_run_invalid_limit`: validates argument parsing
- `test_help`: validates help/usage output
- `test_unknown_subcommand`: validates error handling for unknown subcommands

Each test constructs a temporary program in memory, assembles it, and exercises
the corresponding subcommand end-to-end.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
