# jxcl-protocol

A serde-serializable request/response protocol for remote jxcl-machine control: assemble, run, return trace/final state.

## Purpose

This crate defines the wire protocol types for controlling a jxcl-machine remotely (e.g., over HTTP, gRPC, or another transport layer). Clients serialize `Request`s to JSON, send them to a server, and deserialize `Response`s to learn the execution outcome and final machine state.

## Public API

- **`Request`**: An enum of remote control operations:
  - `AssembleProgram { bytecode: Vec<u8>, memory_size: Option<u64> }` — Load raw bytecode into memory at address 0.
  - `RunProgram { instruction_limit: Option<u64>, enable_trace: bool }` — Execute the loaded program to completion or until the instruction limit is reached.
  - `GetState` — Query the current machine state without executing further instructions.

- **`MachineState`**: A serializable struct capturing the final architectural state after a run:
  - `halted: bool` — Whether the machine halted or is still running.
  - `fault: Option<String>` — Any fault that occurred (e.g., "DivideByZero", "InvalidOpcode"), or `None` if clean.
  - `cycles: u64` — Total number of instructions executed.
  - `pc: u64` — Program counter.
  - `sp: u64` — Stack pointer.
  - `flags: u64` — Flags register.
  - `registers: [u64; 32]` — All 32 general-purpose register values.

- **`Response`**: An enum covering success and error outcomes:
  - `Success { state: Box<MachineState> }` — The operation completed successfully with the final architectural state (boxed for efficient enum layout).
  - `Error { message: String }` — The operation failed with a human-readable error description.

## Dependencies

Workspace crates:

- `jxcl-simulator`
- `jxcl-trace`

External crates:

- `serde` (with `derive` feature)
- `serde_json`

## Implementation Notes

- `Request` and `Response` fully derive `serde::Serialize` and `serde::Deserialize`, allowing them to be transmitted as JSON or any other serde-compatible format.
- `Request` uses `#[serde(tag = "type", content = "data")]` for externally tagged enum representation, so variants serialize as `{ "type": "AssembleProgram", "data": {...} }`.
- `Response` uses `#[serde(tag = "status")]` for internal tagging, so variants appear as `{ "status": "success", "state": {...} }` or `{ "status": "error", "message": "..." }`.
- `MachineState` is a serializable struct that wraps the fields of `RunResult` from `jxcl-simulator` (which does not itself implement serde traits).
- The `state` field in `Response::Success` is boxed (`Box<MachineState>`) to reduce the size difference between enum variants, improving memory efficiency.
- The `fault` field in `MachineState` uses `#[serde(skip_serializing_if = "Option::is_none")]` to omit it from JSON when absent.
- The `memory_size` and `instruction_limit` fields in `Request` are optional; the server provides sensible defaults when not specified.
- No unsafe code is used; the crate is compiled with `#![forbid(unsafe_code)]`.

## Testing

The crate includes 25 tests:

- **Unit tests** (10): Construction and equality checks for each `Request` variant (6 tests) and each `Response` variant (4 tests).
- **Serialization tests** (15): Round-trip serialization/deserialization of each variant to/from JSON, including boundary cases with large register values, fault states, and chained sequences of multiple requests/responses.

All tests ensure that variants can be serialized to JSON and deserialized back to identical Rust values, validating the wire protocol's correctness.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
