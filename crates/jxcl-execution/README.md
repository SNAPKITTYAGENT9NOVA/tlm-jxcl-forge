# jxcl-execution

The fetch/decode/execute loop: owns instruction execution, wiring together decoding, dispatch, the ALU, memory, and exceptions.

## Purpose

Implements the complete fetch/decode/execute cycle (spec §13, §20): reads and validates an instruction from memory, dispatches it to the appropriate semantic handler, updates architectural state, and advances the program counter. Instruction semantics are delegated to specialized execution-group crates (`jxcl-alu`, `jxcl-control`, `jxcl-load-store`, etc.) rather than hard-coded here; this crate's role is orchestration and fault handling.

**Owns:** `step(&mut Machine)` -- the only place an instruction is actually executed.

**Category:** execution · **Source:** extracted from `jxcl/src/execution.rs`

## Public API

### `fn step(state: &mut Machine) -> StepResult`

Execute exactly one instruction. Returns:
- `StepResult::Continued` if the machine is ready to continue
- `StepResult::Halted` if the machine executed HALT (idempotent thereafter)
- `StepResult::Signaled(Signal)` if the machine executed SYS/TRAP
- `StepResult::Faulted(ExecutionFault)` if an architectural fault occurred

Idempotent once halted or faulted.

### `fn run(state: &mut Machine, limit: u64) -> StepResult`

Execute up to `limit` instructions, stopping early if the machine halts, signals, or faults.

## Implementation Notes

### Instruction Coverage

All 47 JXCL opcodes are implemented (spec §20):

- **Data movement:** MOV, MOVI, LOAD, STORE, LEA, PUSH, POP
- **Integer arithmetic:** ADD, SUB, ADC, SBC, MUL, MULH, DIV, REM
- **Logic:** AND, OR, XOR, NAND, NOR
- **Shifts:** SHL, SHR, SAR, ROL, ROR
- **Bit field:** XOR3
- **Comparison:** CMP, TEST
- **Unary arithmetic:** NEG, INC, DEC, NOT
- **Control flow:** Jz, Jnz, Jc, Jnc, Jl, Jle, Jg, Jge, HALT
- **Atomic:** CAS, XCHG, FENCE
- **System:** SYS, TRAP

### Dispatch Architecture

The `step()` function orchestrates a large match statement over `(mnemonic, operands)` pairs, delegating:

- **ALU operations** to `jxcl_alu::*` functions (add, sub, mul, div, and, or, xor, etc.)
- **Control flow** evaluation to `jxcl_control::evaluate_condition` for conditional branches
- **Instruction decoding** to `jxcl_decoding::decode_one`
- **Flag merging** to `jxcl_flags::Flags::apply_effect` to respect instruction-specific FlagEffect masks

### Error Handling

Faults are categorized by type (from `jxcl_errors::ExecutionFault`):

- `InvalidOpcode`: Decoded opcode byte has no handler
- `InvalidRegister`: Register index is out of bounds
- `IllegalOperand`: Instruction is truncated or malformed
- `StackFault`: PUSH/POP would overflow or underflow
- `DivideByZero`: DIV or REM with divisor zero
- (Memory faults mapped from `jxcl_memory::MemoryFault`)

## Testing

7 unit tests covering:

- Normal instruction execution and stepping (add_two_immediates_and_halt, normal_step_continues)
- R0 hardwired-zero semantics (r0_is_hardwired_to_zero)
- Fault handling (divide_by_zero_faults, stack_underflow_on_ret_faults)
- Halt idempotence (step_after_halt_returns_halted)
- Signal handling (trap_signals_without_halting)

Run with `cargo test -p jxcl-execution`.

## Dependencies

Workspace crates:

- `jxcl-machine`: Machine state, registers, program counter
- `jxcl-alu`: ALU operations and result bundles
- `jxcl-control`: Conditional branch evaluation
- `jxcl-decoding`: Instruction decoder
- `jxcl-memory`: Memory access with bounds and alignment checks
- `jxcl-dispatch`: (for internal dispatch logic)
- `jxcl-exceptions`: Fault definitions
- `jxcl-load-store`: (for load/store semantics in future generalization)
- `jxcl-errors`: Error types
- `jxcl-opcodes`: Opcode definitions and lookup
- `jxcl-instructions`: Instruction and operand structures
- `jxcl-registers`: Register file operations
- `jxcl-flags`: Flag register and FlagEffect masks
- `jxcl-types`: Address, Word, RegisterIndex newtypes
- `jxcl-encoding`: Instruction encoding (used in tests only)

External crates:

*(none)*

## Status

Implemented. All opcodes functional, tests passing, clippy and rustfmt checks clean.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
