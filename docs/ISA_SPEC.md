# TLM JXCL — Instruction Set Architecture Specification

This is the authoritative architectural specification for **TLM JXCL**
("Pure Raw Dense ISA Forge"): a deterministic, byte-addressable, 64-bit
instruction set, and the reference implementation of it in `src/`.

The implementation (`isa::opcodes::ALL`/`all_defs()`) is the single
source of truth for opcode numbers, formats and flag effects; this
document is a human-readable rendering of that same table plus the
architectural decisions the code makes (this file and `isa/opcodes.rs`
must never disagree — if they do, the registry is correct and this file
has a bug).

## 1. Machine model

| Component | Width | Notes |
|---|---|---|
| General-purpose registers `R0`-`R31` | 64-bit | `R0` is hardwired to zero: reads yield `0`, writes are discarded. |
| `PC` (program counter) | 64-bit | Holds the address of the instruction currently being fetched. |
| `SP` (stack pointer) | 64-bit | Grows downward; initialized to `memory.len()`. |
| `FP` (frame pointer) | 64-bit | Architectural register; the base ISA never writes it itself (available for calling-convention use by software). |
| `FLAGS` | 4 used bits | `Z` (bit 0), `N` (bit 1), `C` (bit 2), `V` (bit 3). |
| Address space | 64-bit | Backed by a bounded flat buffer in the reference machine. |
| Endianness | Little-endian | All multi-byte memory and immediate fields. |

## 2. Encoding

Every register operand occupies **one whole byte** (values `0..32` are
legal; anything else is `INVALID_REGISTER`). This is a deliberate
trade-off: JXCL is dense relative to fixed-width RISC encodings (the
shortest instructions are 1-2 bytes; nothing is padded to a uniform
word), but every field boundary is byte-aligned, so decoding is never
ambiguous — no bit-packing puzzle, no variable-length integer scheme.
Immediates and displacements are fixed-width per opcode (never chosen by
a runtime flag) and little-endian.

| Format | Layout | Length |
|---|---|---|
| `None` | `opcode` | 1 |
| `R` | `opcode, rd` | 2 |
| `RR` | `opcode, rd, rs` | 3 |
| `RRR` | `opcode, rd, rs1, rs2` | 4 |
| `RImm64` | `opcode, rd, imm64` | 10 |
| `RMem` | `opcode, rd, base, disp32` | 7 |
| `MemR` | `opcode, base, disp32, rs` | 7 |
| `Cas` | `opcode, rd, base, rs_new, disp32` | 8 |
| `BranchImm32` | `opcode, disp32` | 5 |
| `Imm16` | `opcode, imm16` | 3 |

Minimum instruction length: 1 byte. Maximum: 10 bytes. Opcode space: one
byte (256 possible opcodes; 50 are assigned).

Effective address for every memory operand (`RMem`/`MemR`/`Cas`) is
`R[base] + sign_extend(disp32)` — there is no scaled-index
(`base + index*scale + disp`) addressing mode in this version; that is a
deliberate scope decision (see spec discussion §8), not an oversight.

## 3. Opcode map

| Opcode | Mnemonic | Format | Flags | Category |
|---|---|---|---|---|
| 0x00 | NOP | None | — | System |
| 0x01 | MOV | RR | — | Data |
| 0x02 | MOVI | RImm64 | — | Data |
| 0x03 | LOAD | RMem | — | Data |
| 0x04 | STORE | MemR | — | Data |
| 0x05 | PUSH | R | — | Data |
| 0x06 | POP | R | — | Data |
| 0x07 | LEA | RMem | — | Data |
| 0x10 | ADD | RR | Z,N,C,V | Arithmetic |
| 0x11 | SUB | RR | Z,N,C,V | Arithmetic |
| 0x12 | ADC | RR | Z,N,C,V | Arithmetic |
| 0x13 | SBC | RR | Z,N,C,V | Arithmetic |
| 0x14 | MUL | RR | Z,N,C,V | Arithmetic |
| 0x15 | MULH | RR | Z,N | Arithmetic |
| 0x16 | DIV | RR | Z,N | Arithmetic |
| 0x17 | REM | RR | Z,N | Arithmetic |
| 0x18 | NEG | R | Z,N,C,V | Arithmetic |
| 0x19 | INC | R | Z,N,V | Arithmetic |
| 0x1A | DEC | R | Z,N,V | Arithmetic |
| 0x20 | AND | RR | Z,N | Logical |
| 0x21 | OR | RR | Z,N | Logical |
| 0x22 | XOR | RR | Z,N | Logical |
| 0x23 | NOT | R | Z,N | Logical |
| 0x24 | NAND | RR | Z,N | Logical |
| 0x25 | NOR | RR | Z,N | Logical |
| 0x26 | XOR3 | RRR | Z,N | Logical |
| 0x30 | SHL | RR | Z,N,C | Shift |
| 0x31 | SHR | RR | Z,N,C | Shift |
| 0x32 | SAR | RR | Z,N,C | Shift |
| 0x33 | ROL | RR | Z,N,C | Shift |
| 0x34 | ROR | RR | Z,N,C | Shift |
| 0x40 | CMP | RR | Z,N,C,V | Comparison |
| 0x41 | TEST | RR | Z,N | Comparison |
| 0x50 | JMP | BranchImm32 | — | Control flow |
| 0x51 | CALL | BranchImm32 | — | Control flow |
| 0x52 | RET | None | — | Control flow |
| 0x53 | JZ | BranchImm32 | — | Control flow |
| 0x54 | JNZ | BranchImm32 | — | Control flow |
| 0x55 | JC | BranchImm32 | — | Control flow |
| 0x56 | JNC | BranchImm32 | — | Control flow |
| 0x57 | JL | BranchImm32 | — | Control flow |
| 0x58 | JLE | BranchImm32 | — | Control flow |
| 0x59 | JG | BranchImm32 | — | Control flow |
| 0x5A | JGE | BranchImm32 | — | Control flow |
| 0x60 | HALT | None | — | System |
| 0x61 | TRAP | Imm16 | — | System |
| 0x62 | SYS | Imm16 | — | System |
| 0x70 | CAS | Cas | Z,N (custom, see §7) | Atomic |
| 0x71 | XCHG | RMem | — | Atomic |
| 0x72 | FENCE | None | — | Atomic |

`isa::opcodes::all_defs()` is machine-readable proof this table is
exhaustive and collision-free (`no_duplicate_opcodes` /
`every_mnemonic_has_exactly_one_entry` tests enforce it on every build).

## 4. Flag semantics

- **Carry (`C`)** follows the x86 convention: for `ADD`/`ADC` it is the
  *unsigned carry-out* of bit 63; for `SUB`/`SBC`/`CMP`/`NEG` it is the
  unsigned *borrow* (`1` iff the true difference is negative).
- **Overflow (`V`)** is always the *signed* overflow of the operation.
- `INC`/`DEC` leave `C` unchanged (only `Z`, `N`, `V`) — a documented
  choice distinguishing them from full `ADD`/`SUB` of `1`.
- `MUL` sets `C` and `V` together when the discarded high 64 bits of the
  128-bit product are nonzero; it does not distinguish signed vs.
  unsigned overflow.
- Shift/rotate `C` is the last bit shifted out (or, for shift-by-zero,
  `false`). Shift/rotate counts are masked to the low 6 bits of the
  count operand (`0..64`), never faulting on a large count.
- `CMP`/`TEST` compute like `SUB`/`AND` respectively but discard the
  result, keeping only flags.

## 5. Memory model

The reference machine is a single flat, bounded address space split into
two regions:

- **Code region** `[0, code_size)`: readable + executable, **not**
  writable.
- **Everything else**: readable + writable, **not** executable.

An access that only *partially* overlaps the code region is denied in
full (not just the overlapping part) — matching how a real MMU denies an
access that touches any byte of a protected page.

16/32/64-bit accesses must be naturally aligned or raise
`AlignmentFault`; 8-bit accesses are always aligned. Faults:
`INVALID_ADDRESS`, `ALIGNMENT_FAULT`, `READ_VIOLATION`,
`WRITE_VIOLATION`, `EXECUTE_VIOLATION` (`memory.rs`), plus
`INVALID_OPCODE`, `INVALID_REGISTER`, `DIVIDE_BY_ZERO`, `STACK_FAULT`,
`ILLEGAL_OPERAND` at the execution layer (`execution.rs`).

`LOAD`/`STORE`/`XCHG`/`CAS` all operate on 64-bit words only in this
version (no byte/half-word/word memory ops) — the enumerated instruction
list never asked for width-suffixed memory ops, so this is a scope
decision, not an omission.

## 6. Control flow

`PC` holds the address of the instruction currently being fetched.
Every branch/`CALL` displacement is **relative to the address of the
next sequential instruction**:

```
pc_after_fetch = pc_of_instruction + instruction_length
target = pc_after_fetch + displacement
```

Signed conditions after a `CMP`/`SUB` use the standard two's-complement
flag algebra: `JL` is `N != V`, `JGE` is `N == V`, `JG` is
`!Z && (N == V)`, `JLE` is `Z || (N != V)`.

`CALL` pushes `pc_after_fetch` (the return address) and jumps to the
target; `RET` pops that address back into `PC`. The stack grows
downward: `PUSH` decrements `SP` by 8 before writing; `POP` reads then
increments `SP` by 8. `SP` is initialized to `memory.len()`.

## 7. Atomics (single-threaded reference semantics)

`CAS rd, [base+disp], rs_new`: reads the memory word at the effective
address; if it equals `R[rd]` ("expected"), writes `R[rs_new]` there and
leaves `R[rd]` unchanged, setting `Z`. Otherwise `R[rd]` is overwritten
with the actual memory value and `Z` is cleared. `N` reflects the sign
bit of whatever ends up in `R[rd]`. This is a custom flag convention
(not the generic ALU `Z`/`N`), documented here as required by spec §25.

`XCHG rd, [base+disp]`: atomically swaps `R[rd]` with the memory word.

`FENCE`: a real, decodable no-op in this single-threaded reference
model — there is no second agent whose memory-visibility order a fence
could affect. It exists so instruction streams written for a concurrent
JXCL backend still assemble and execute here unchanged (spec §25
explicitly disclaims any hardware cache-coherency claim beyond what is
actually implemented).

## 8. Binary format

See `src/binary.rs` for the exact 56-byte header layout (`MAGIC`,
`VERSION`, `ARCHITECTURE`, `ENTRY_POINT`, `CODE_OFFSET`, `CODE_SIZE`,
`DATA_OFFSET`, `DATA_SIZE`, `FLAGS`, reserved). When loaded, code is
placed at virtual address `0` and data immediately follows at virtual
address `code_size` — a fixed, deterministic layout.

## 9. Assembly syntax

```
start:
    MOVI R1, 10
    MOVI R2, 20
loop:
    ADD  R1, R2
    DEC  R2
    CMP  R2, R0
    JG   loop
    HALT

.data
buffer:
    .qword 1234
    .byte 1, 2, 3
```

- Labels (`name:`), instructions, and directives are one per line;
  `;` starts a line comment.
- Registers: `R0`-`R31` (case-insensitive).
- Numeric literals: decimal, `0x` hex, `0b` binary; a leading `+`/`-` is
  accepted before a literal.
- Memory operands: `[Rbase]`, `[Rbase+disp]`, `[Rbase-disp]`.
- Directives: `.code`/`.text`, `.data` (switch section), `.const NAME
  value`, `.entry LABEL_OR_NUMBER`, `.byte`/`.word`/`.dword`/`.qword
  v, v, ...` (emit data).
- A label or `.const` reference is used wherever an immediate,
  displacement, or branch target is expected; label addresses are
  absolute virtual addresses (code labels equal their code offset; data
  labels equal `code_size + their data offset`).

The disassembler's canonical form never invents label names — it prints
every resolved address as a plain number, which the assembler accepts
anywhere it accepts a label. This is what makes
`assemble(disassemble(program)) == program` hold without needing a
symbolication pass in the disassembler (see `src/disassembler.rs`).

## 10. Execution engine

`execution::step` performs, in order: **fetch** (opcode byte, then the
rest of the instruction once its format-determined length is known) →
**decode** (`encoding::decoder::decode_one`, which also validates every
register field) → **execute** (the giant match in `execution.rs`,
dispatching to `alu.rs` for arithmetic/logical/shift operations) →
**update state** → **next PC**. Every architectural fault path sets
`state.fault` and `state.halted` and returns `StepResult::Faulted`
rather than panicking. The handful of `.unwrap()`/`.expect()` calls
outside test code (`execution.rs`'s register read/write, `main.rs`'s
post-validation binary re-parse, fixed-width slice-to-array conversions
in `binary.rs`/`encoding/decoder.rs` after a length check earlier in the
same function) are all on invariants the immediately preceding code in
that same function already established — none of them is reachable on
malformed input reaching that point, but they are not architectural
fault handling either. All *architectural* error/fault reporting — bad
opcodes, bad registers, memory violations, divide-by-zero, stack
under/overflow, malformed assembly/binaries — goes through the
structured error types in `errors.rs`, never a panic.

`SYS`/`TRAP` never touch the host filesystem/network/process/environment
themselves — they only set `state.pending_signal`, which the CLI or a
future host integration observes after `step()` returns.
