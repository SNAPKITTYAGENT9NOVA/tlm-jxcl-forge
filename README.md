# tlm-jxcl-forge
TLM JXCL PURE RAW DENSE ISA FORGE - 3500-line implementation specification

## TLM JXCL

A complete, deterministic, byte-addressable, 64-bit instruction-set
architecture and toolchain: opcode registry, encoder, decoder,
reference execution engine, ALU, memory subsystem, register/flag
subsystem, binary format, static validator, assembler, disassembler,
CLI, and a debugger/trace mode — plus unit, property, golden-vector, and
decoder-fuzz test suites. No external dependencies.

- **Full architecture spec:** [`docs/ISA_SPEC.md`](./docs/ISA_SPEC.md)
- **Hardware/RTL integration contract:** [`docs/RTL_CONTRACT.md`](./docs/RTL_CONTRACT.md)
- **Example program:** [`examples/loop.jxcl`](./examples/loop.jxcl)

```
cargo build --release
cargo test

target/release/jxcl asm examples/loop.jxcl -o loop.jxc
target/release/jxcl validate loop.jxc
target/release/jxcl disasm loop.jxc
target/release/jxcl run loop.jxc --trace
```

`jxcl` subcommands: `asm <in.jxcl> -o <out.jxc>`, `disasm <program.jxc>`,
`run <program.jxc> [--trace] [--limit N]`, `inspect <program.jxc>`,
`validate <program.jxc>`.

Source layout: `src/isa/` (constants, registers, flags, opcode registry,
operand model), `src/encoding/` (encoder/decoder), `src/alu.rs`,
`src/memory.rs`, `src/machine.rs` + `src/execution.rs` (machine state and
the fetch/decode/execute engine), `src/control.rs` (branch semantics),
`src/binary.rs` + `src/validator.rs` (container format and static
validation), `src/assembler/` (lexer/parser/two-pass assembler),
`src/disassembler.rs`, `src/debugger.rs` (trace mode), `src/main.rs`
(CLI). Tests live both inline (`#[cfg(test)]` per module) and in
`tests/` (property tests, golden vectors, decoder fuzz).

## redis-implementation-js

A small Node.js/Express demo illustrating Redis-backed HTTP response caching, merged in from the separate `Redis-implementation-js` repository. See [`redis-implementation-js/`](./redis-implementation-js) for the code:

- `server.js` — plain Express server (port 3000) that fetches https://jsonplaceholder.typicode.com/photos on every request.
- `server-cached.js` — same idea (port 3001) but backed by Redis (`redis://127.0.0.1:6379`), caching the response for 1 hour to demonstrate cache-hit vs cache-miss latency.

Run with `node redis-implementation-js/server.js` or `node redis-implementation-js/server-cached.js` (the cached variant needs a local Redis instance).
