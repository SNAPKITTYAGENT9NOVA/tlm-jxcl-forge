# RTL Integration Contract

Spec §36-38 ask for an architectural boundary that a future RTL
(SystemVerilog/Verilog/VHDL) implementation can be built against without
either side needing to change the other's architectural semantics. This
document is that boundary, plus how the golden test vectors in
`tests/golden_vectors.rs` become hardware test vectors.

**This repository does not contain RTL.** It contains the reference
software model and the contract a future RTL implementation must honor.
Claiming otherwise — presenting the Rust emulator as if it were, or
implied fabricated hardware — is exactly what spec §35 forbids ("Do not
pretend that a software emulator itself constitutes fabricated
hardware").

## Conceptual pipeline mapping

| JXCL concept | Hardware unit | Reference implementation |
|---|---|---|
| Fetch (opcode + operand bytes at `PC`) | Instruction Fetch unit | `memory::Memory::fetch`, `execution::step`'s fetch stage |
| Decode | Decode / instruction-format ROM | `encoding::decoder::decode_one`, keyed by `isa::opcodes` |
| Register read/write | Register File | `isa::registers::RegisterFile` |
| ADD/SUB/ADC/SBC/MUL/MULH/DIV/REM/NEG/INC/DEC | Integer ALU / multiplier / divider | `alu.rs` |
| AND/OR/XOR/NOT/NAND/NOR/XOR3 | Logic gates | `alu.rs` |
| SHL/SHR/SAR/ROL/ROR | Barrel shifter | `alu.rs` |
| CMP/TEST | ALU + flag generation, result discarded | `alu::cmp`/`alu::test` |
| LOAD/STORE/LEA/PUSH/POP | Address generation + memory interface | `execution.rs`, `memory.rs` |
| JMP/CALL/RET/Jcc | Control-flow unit (+ stack for CALL/RET) | `control.rs`, `execution.rs` |
| CAS/XCHG/FENCE | Atomic unit / memory ordering | `execution.rs` (single-threaded reference semantics, spec §25) |
| Writeback | Register file write port | End of `execution::step`'s match arm |

## What the RTL contract freezes

An RTL implementation of TLM JXCL must reproduce, cycle-independent of
its own microarchitecture:

1. **Every opcode's binary encoding** — `isa::opcodes::all_defs()` (opcode
   byte, `Format`, byte layout per `docs/ISA_SPEC.md` §2-3). This is
   frozen; adding an opcode is fine, changing an existing one's encoding
   is an incompatible architecture revision (bump `ARCHITECTURE_ID` in
   `isa/constants.rs`).
2. **Every instruction's formal semantics** — the match arm in
   `execution.rs` for that mnemonic, including exactly which flags it
   touches (`isa::opcodes::InstructionDef::flags`) and the precise
   carry/overflow/shift/comparison rules in `alu.rs`'s doc comments.
3. **The fault catalog** — `errors::ExecutionFault` / `MemoryFault` and
   exactly which condition raises which fault (`docs/ISA_SPEC.md` §5).
4. **PC/branch semantics** — `control::branch_target`'s formula.
5. **The stack/calling convention** — downward-growing, `SP` semantics
   in `docs/ISA_SPEC.md` §6.

What it does **not** freeze: cycle count, pipeline depth, branch
prediction, cache behavior, or any other microarchitectural detail not
observable in the architectural state (`MachineState`). Spec §34 is
explicit that the reference implementation prioritizes correctness and
clarity over performance, and that applies equally to a hardware
implementation's *internal* timing — only the architecturally visible
end state after each instruction is part of this contract.

## Golden vectors as hardware test vectors

`tests/golden_vectors.rs` defines `Golden { name, source, expect_halted,
expect_fault, expect_registers, expect_flags }` entries. Each one is,
by construction, exactly the shape spec §38 asks for:

- **initial registers / initial memory**: implicit — every vector starts
  from `MachineState::new` (all registers zero, `SP = memory.len()`,
  flags clear) plus whatever the program's own prologue sets up.
- **program bytes**: the assembled `source`.
- **expected final registers / flags / halt-or-fault**: the `expect_*`
  fields.

To turn one into an RTL test bench vector: assemble `source` with
`jxcl asm`, load the resulting `.jxc`'s code section into the RTL
model's instruction memory at address `0`, run the RTL model to
completion (or the same fault), and diff its final register
file/flags/halt-or-fault-code against the `expect_*` fields — byte for
byte, bit for bit. A mismatch means either the RTL or this reference
model has an architectural bug; per spec §36, the ISA specification
(this document plus `ISA_SPEC.md`) — not either implementation — is the
tiebreaker for which one is wrong.
