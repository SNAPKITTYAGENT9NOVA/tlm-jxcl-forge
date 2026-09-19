# tlm-jxcl-forge
TLM JXCL PURE RAW DENSE ISA FORGE - 3500-line implementation specification

A Cargo workspace with four crates, all Rust, zero external dependencies
in the ISA forge and real, audited dependencies everywhere they add
genuine value (async HTTP/Redis, post-quantum cryptography):

| Crate | What it is |
|---|---|
| [`crates/jxcl`](./crates/jxcl) | The TLM JXCL instruction-set architecture: opcode registry, encoder/decoder, ALU, memory, execution engine, binary format, validator, assembler, disassembler, CLI. Zero dependencies. |
| [`crates/pq-crypto`](./crates/pq-crypto) | Post-quantum envelope encryption: ML-KEM-768 (NIST FIPS 203) + HKDF-SHA256 + AES-256-GCM. |
| [`crates/pq-cache`](./crates/pq-cache) | A Redis-backed cache whose entries are sealed with `pq-crypto` before they ever reach Redis. |
| [`crates/photo-cache-service`](./crates/photo-cache-service) | Rust port of the original `server.js`/`server-cached.js` Express+Redis demo, using `pq-cache`. |

## TLM JXCL (`crates/jxcl`)

A complete, deterministic, byte-addressable, 64-bit instruction-set
architecture and toolchain: opcode registry, encoder, decoder,
reference execution engine, ALU, memory subsystem, register/flag
subsystem, binary format, static validator, assembler, disassembler,
CLI, and a debugger/trace mode — plus unit, property, golden-vector, and
decoder-fuzz test suites. No external dependencies.

- **Full architecture spec:** [`docs/ISA_SPEC.md`](./docs/ISA_SPEC.md)
- **Hardware/RTL integration contract:** [`docs/RTL_CONTRACT.md`](./docs/RTL_CONTRACT.md)
- **Example program:** [`crates/jxcl/examples/loop.jxcl`](./crates/jxcl/examples/loop.jxcl)

```
cargo build --release -p jxcl
cargo test -p jxcl

target/release/jxcl asm crates/jxcl/examples/loop.jxcl -o loop.jxc
target/release/jxcl validate loop.jxc
target/release/jxcl disasm loop.jxc
target/release/jxcl run loop.jxc --trace
```

`jxcl` subcommands: `asm <in.jxcl> -o <out.jxc>`, `disasm <program.jxc>`,
`run <program.jxc> [--trace] [--limit N]`, `inspect <program.jxc>`,
`validate <program.jxc>`.

Source layout (all under `crates/jxcl/`): `src/isa/` (constants,
registers, flags, opcode registry, operand model), `src/encoding/`
(encoder/decoder), `src/alu.rs`, `src/memory.rs`, `src/machine.rs` +
`src/execution.rs` (machine state and the fetch/decode/execute engine),
`src/control.rs` (branch semantics), `src/binary.rs` + `src/validator.rs`
(container format and static validation), `src/assembler/`
(lexer/parser/two-pass assembler), `src/disassembler.rs`,
`src/debugger.rs` (trace mode), `src/main.rs` (CLI). Tests live both
inline (`#[cfg(test)]` per module) and in `tests/` (property tests,
golden vectors, decoder fuzz).

## Post-quantum Redis cache (`pq-crypto` / `pq-cache` / `photo-cache-service`)

A Rust port of the original `redis-implementation-js` demo (an
Express server illustrating Redis-backed HTTP response caching),
hardened for production and split into three crates so the
post-quantum cryptography is an independent, fully unit-tested
building block rather than something bolted onto the HTTP layer:

- **`pq-crypto`** implements ML-KEM-768 (the NIST FIPS 203 standardized
  post-quantum key encapsulation mechanism, formerly CRYSTALS-Kyber) +
  HKDF-SHA256 + AES-256-GCM as a hybrid envelope encryption scheme. See
  its module docs for the full construction and threat model.
- **`pq-cache`** wraps an async Redis client so that every value is
  sealed with `pq-crypto` before being written and opened after being
  read — Redis itself never sees plaintext. A corrupted or
  undecryptable entry degrades to a cache miss rather than an error.
- **`photo-cache-service`** provides two binaries mirroring the
  original demo: `server` (uncached, port 3000 by default) and
  `server-cached` (PQ-encrypted-cache-backed, port 3001 by default),
  both exposing `GET /photos` and `GET /healthz`, with structured
  `tracing` logs, env-var configuration, request timeouts, and
  graceful shutdown on Ctrl-C/SIGTERM.

```
cargo build --release -p photo-cache-service

# start a local Redis (or point REDIS_URL at an existing one)
redis-server --port 6379 &

PORT=3000 ./target/release/server &
PORT=3001 REDIS_URL=redis://127.0.0.1:6379 CACHE_TTL_SECONDS=3600 ./target/release/server-cached &

curl localhost:3000/photos   # always fetches upstream
curl localhost:3001/photos   # first call: MISS (fetches + seals into Redis)
curl localhost:3001/photos   # second call: HIT (opens the sealed entry)
```

Config (all optional, shown with defaults): `PORT` (3000 / 3001),
`REDIS_URL` (`redis://127.0.0.1:6379`, `server-cached` only),
`CACHE_TTL_SECONDS` (3600, `server-cached` only), `RUST_LOG` (`info`).

See [`docs/HARDENING.md`](./docs/HARDENING.md) for the production
hardening checklist and the post-quantum scheme's threat model/scope.
