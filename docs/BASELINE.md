# Baseline (pre-100-crate-expansion)

Recorded before any expansion work began, per the "preserve the existing
six" rule. This is the reference point every later change is validated
against: nothing in the expansion may reduce test count, weaken public
APIs, or regress these gates without an explicit, documented migration.

Commit: `a270e35` ("Add key rotation, TLS enforcement, a SQL Server
vault, and verifiable error attestations"), branch
`claude/pull-request-two-repos-kcuagc`, based on `origin/main` after
PR #4 merged.

## Workspace (6 crates)

1. `crates/jxcl` — 4,840 lines across `src/`, plus `tests/` (property
   tests, golden vectors, decoder fuzz). Zero external dependencies.
2. `crates/pq-crypto` — 520 lines. ML-KEM-768 + HKDF-SHA256 +
   AES-256-GCM, `KeyRing` rotation.
3. `crates/pq-cache` — 295 lines. Redis-backed encrypted cache.
4. `crates/pq-sql-vault` — 441 lines + 4 SQL migration files. SQL
   Server-backed encrypted vault via `tiberius`.
5. `crates/pq-error-proof` — 456 lines. Groth16 zk-SNARK (arkworks)
   error-attestation proofs.
6. `crates/photo-cache-service` — 524 lines. Two axum binaries
   (`server`, `server-cached`).

Total: 7,076 lines of Rust across the workspace (excluding generated
`Cargo.lock`).

## Gate results

| Gate | Command | Result |
|---|---|---|
| G1 fmt | `cargo fmt --all -- --check` | pass (exit 0) |
| G2 check | `cargo check --workspace --all-targets` | pass (exit 0), 28.28s |
| G3 test | `cargo test --workspace --release` | pass: **106 passed**, 0 failed, 2 ignored (`pq-sql-vault`'s live-SQL-Server tests, gated on an env var — no SQL Server in this environment) |
| G4 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass (exit 0) |
| `cargo audit` | (see `docs/HARDENING.md`) | 0 vulnerabilities as of the last audit; 1 unmaintained-crate warning (`derivative`, transitive via `ark-crypto-primitives`, upstream-only) |

## Environment constraints noted for the expansion

- No `iverilog`, `verilator`, or `yosys` available in this environment
  (checked via `which`). The Hardware/RTL crate batch cannot run real
  Verilog simulation or synthesis; it is scoped to real HDL-AST
  construction, text-backend codegen (Verilog/VHDL), a toy structural
  synthesis pass, and mechanical cross-checks against the actual
  `jxcl-opcodes`/`jxcl-constants` source of truth (golden-file
  regression + structural assertions), not functional hardware
  simulation. This boundary is documented in
  `docs/HARDWARE_LIMITATIONS.md`.
- No live SQL Server or Docker daemon available (same constraint as the
  original `pq-sql-vault` work) — its live integration tests stay
  `#[ignore]`d.
- GitHub is reachable via the GitHub MCP server for this session, but
  direct `git clone`/HTTPS access to arbitrary GitHub hosts remains
  policy-gated per the environment's egress rules; all crates.io
  dependency work in this expansion uses only what's already vendored
  in the local cargo registry cache or freshly resolvable from
  crates.io.

## `jxcl`'s existing module map (ground truth for the ISA/Execution/Toolchain/Debug splits)

```
src/lib.rs                 (17 lines)  — crate root, module declarations
src/errors.rs              (188 lines) — crate-wide error type
src/isa/mod.rs              (13 lines) — ISA layer root
src/isa/constants.rs        (75 lines)
src/isa/registers.rs        (79 lines)
src/isa/flags.rs           (115 lines)
src/isa/operand.rs         (122 lines)
src/isa/instruction.rs      (26 lines)
src/isa/opcodes.rs         (299 lines)
src/encoding/mod.rs          (4 lines)
src/encoding/encoder.rs    (119 lines)
src/encoding/decoder.rs    (162 lines)
src/alu.rs                 (433 lines)
src/memory.rs               (282 lines)
src/machine.rs              (116 lines)
src/execution.rs            (533 lines)
src/control.rs               (68 lines)
src/binary.rs               (213 lines)
src/validator.rs            (138 lines)
src/assembler/mod.rs        (520 lines)
src/assembler/lexer.rs      (191 lines)
src/assembler/parser.rs     (425 lines)
src/disassembler.rs         (210 lines)
src/debugger.rs             (177 lines)
src/main.rs                 (315 lines) — CLI
tests/property_tests.rs, tests/golden_vectors.rs, tests/fuzz_decoder.rs, tests/common/mod.rs
```

This map is the direct source for the Foundation/ISA/Execution/
Memory/Toolchain/Debug crate extractions in
`docs/CRATE_ARCHITECTURE.md` — each new crate's initial content is
real code moved from one of these files, not invented from scratch,
except where the plan explicitly calls out new functionality (object
format/linker, RTL codegen, protocol/RPC, proof registry, storage
journal/ledger/migration — see `docs/CRATE_GENERATION_PLAN.md` for
which crates are "extraction" vs. "new").
