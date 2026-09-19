# jxcl-exceptions

The machine's exception model: illegal opcode, misaligned access, division by zero, and the trap-handling hook.

## Architecture

**Owns:** The `Exception` enum and the `TrapHandler` trap-dispatch contract.

**Category:** execution · **Source:** new (generalizes
`crates/jxcl/src/errors.rs`'s `ExecutionFault` into the shared vocabulary
of exceptions the execution-group and memory-group crates -- page table,
stack, heap, execution -- all need to signal).

## Public API

- `Exception` -- `IllegalOpcode`, `InvalidRegister`, `IllegalOperand`,
  `MisalignedAccess`, `DivisionByZero`, `StackOverflow`,
  `StackUnderflow`, `PageFault`, `PermissionViolation`, `Memory(..)`
  (wraps `jxcl-errors::MemoryFault`)
- `TrapHandler` -- a trait a host implements to decide `TrapAction::Halt`
  or `TrapAction::Resume` after observing an exception
- `HaltOnException` / `RecordingTrapHandler` -- two ready-made handlers

## Dependencies

Workspace crates:

- `jxcl-types`
- `jxcl-errors`

External crates:

*(none)*

## Testing

`cargo test -p jxcl-exceptions`: unit tests for the `MemoryFault` ->
`Exception` conversion, `Display` formatting (error-path), and both
built-in `TrapHandler` implementations.

## Status

`implemented`.
