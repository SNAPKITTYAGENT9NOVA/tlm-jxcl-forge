# Dependency Graph

Generated from `docs/crates.toml` -- do not hand-edit; regenerate with
`python3 tools/gen_dependency_graph.py` if the registry changes.

Validated acyclic and free of forbidden ISA-to-storage/network edges by
`tools/check_workspace.py` (see its output in `docs/BUILD_MATRIX.md`).

## Foundation (8)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-bitops` | `jxcl-types` | *(none)* |
| `jxcl-bytes` | `jxcl-errors`, `jxcl-endian` | *(none)* |
| `jxcl-config` | `jxcl-errors` | *(none)* |
| `jxcl-constants` | *(none)* | *(none)* |
| `jxcl-endian` | *(none)* | *(none)* |
| `jxcl-errors` | `jxcl-types` | *(none)* |
| `jxcl-logging` | *(none)* | `tracing`, `tracing-subscriber` |
| `jxcl-types` | *(none)* | *(none)* |

## ISA (12)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl` | `jxcl-opcodes`, `jxcl-registers`, `jxcl-flags`, `jxcl-operands`, `jxcl-instructions`, `jxcl-encoding`, `jxcl-decoding`, `jxcl-alu`, `jxcl-memory`, `jxcl-machine`, `jxcl-execution`, `jxcl-control`, `jxcl-binary`, `jxcl-instruction-validation`, `jxcl-assembler`, `jxcl-disassembler`, `jxcl-debugger`, `jxcl-cli` | *(none)* |
| `jxcl-decoding` | `jxcl-instructions`, `jxcl-bitops`, `jxcl-endian`, `jxcl-bytes`, `jxcl-errors` | *(none)* |
| `jxcl-encoding` | `jxcl-instructions`, `jxcl-bitops`, `jxcl-endian`, `jxcl-bytes`, `jxcl-errors` | *(none)* |
| `jxcl-flags` | `jxcl-types` | *(none)* |
| `jxcl-instruction-validation` | `jxcl-instructions`, `jxcl-operands`, `jxcl-registers`, `jxcl-errors` | *(none)* |
| `jxcl-instructions` | `jxcl-opcodes`, `jxcl-operands` | *(none)* |
| `jxcl-isa-metadata` | `jxcl-constants`, `jxcl-opcodes` | *(none)* |
| `jxcl-isa-schema` | `jxcl-opcodes`, `jxcl-constants`, `jxcl-registers`, `jxcl-flags` | `serde` |
| `jxcl-isa-versioning` | `jxcl-constants` | *(none)* |
| `jxcl-opcodes` | `jxcl-types`, `jxcl-constants` | *(none)* |
| `jxcl-operands` | `jxcl-types`, `jxcl-registers` | *(none)* |
| `jxcl-registers` | `jxcl-types`, `jxcl-constants` | *(none)* |

## Execution (10)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-alu` | `jxcl-types`, `jxcl-flags`, `jxcl-errors` | *(none)* |
| `jxcl-branch` | `jxcl-types`, `jxcl-operands` | *(none)* |
| `jxcl-control` | `jxcl-types`, `jxcl-flags`, `jxcl-registers` | *(none)* |
| `jxcl-cycle-model` | `jxcl-opcodes` | *(none)* |
| `jxcl-dispatch` | `jxcl-instructions`, `jxcl-opcodes` | *(none)* |
| `jxcl-exceptions` | `jxcl-types`, `jxcl-errors` | *(none)* |
| `jxcl-execution` | `jxcl-machine`, `jxcl-alu`, `jxcl-control`, `jxcl-decoding`, `jxcl-memory`, `jxcl-dispatch`, `jxcl-exceptions`, `jxcl-load-store`, `jxcl-errors` | *(none)* |
| `jxcl-interrupts` | `jxcl-flags`, `jxcl-exceptions` | *(none)* |
| `jxcl-machine` | `jxcl-registers`, `jxcl-flags`, `jxcl-types` | *(none)* |
| `jxcl-pipeline` | `jxcl-instructions`, `jxcl-cycle-model` | *(none)* |

## Memory (8)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-address-space` | `jxcl-memory`, `jxcl-types` | *(none)* |
| `jxcl-cache-model` | `jxcl-memory`, `jxcl-address-space` | *(none)* |
| `jxcl-heap` | `jxcl-address-space`, `jxcl-load-store`, `jxcl-exceptions` | *(none)* |
| `jxcl-load-store` | `jxcl-memory`, `jxcl-endian`, `jxcl-exceptions` | *(none)* |
| `jxcl-memory` | `jxcl-types`, `jxcl-errors`, `jxcl-endian` | *(none)* |
| `jxcl-memory-map` | `jxcl-address-space`, `jxcl-constants` | *(none)* |
| `jxcl-page-table` | `jxcl-address-space`, `jxcl-exceptions` | *(none)* |
| `jxcl-stack` | `jxcl-load-store`, `jxcl-exceptions`, `jxcl-registers` | *(none)* |

## Binary/Toolchain (12)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-assembler` | `jxcl-lexer`, `jxcl-parser`, `jxcl-object`, `jxcl-symbols`, `jxcl-relocations`, `jxcl-encoding`, `jxcl-instructions`, `jxcl-linker`, `jxcl-debug-info` | *(none)* |
| `jxcl-binary` | `jxcl-bytes`, `jxcl-endian`, `jxcl-errors`, `jxcl-isa-versioning` | *(none)* |
| `jxcl-cli` | `jxcl-assembler`, `jxcl-disassembler`, `jxcl-loader`, `jxcl-simulator`, `jxcl-debugger`, `jxcl-isa-metadata`, `jxcl-instruction-validation`, `jxcl-config`, `jxcl-logging` | *(none)* |
| `jxcl-debug-info` | `jxcl-symbols`, `jxcl-bytes` | *(none)* |
| `jxcl-disassembler` | `jxcl-decoding`, `jxcl-instructions`, `jxcl-symbols`, `jxcl-debug-info` | *(none)* |
| `jxcl-lexer` | `jxcl-errors` | *(none)* |
| `jxcl-linker` | `jxcl-object`, `jxcl-symbols`, `jxcl-relocations`, `jxcl-binary`, `jxcl-errors` | *(none)* |
| `jxcl-loader` | `jxcl-binary`, `jxcl-memory-map`, `jxcl-memory`, `jxcl-errors` | *(none)* |
| `jxcl-object` | `jxcl-bytes`, `jxcl-symbols`, `jxcl-relocations`, `jxcl-errors` | *(none)* |
| `jxcl-parser` | `jxcl-lexer`, `jxcl-instructions`, `jxcl-operands`, `jxcl-errors` | *(none)* |
| `jxcl-relocations` | `jxcl-symbols`, `jxcl-encoding`, `jxcl-bitops` | *(none)* |
| `jxcl-symbols` | `jxcl-types` | *(none)* |

## Debug/Simulation (8)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-debugger` | `jxcl-machine`, `jxcl-execution`, `jxcl-trace` | *(none)* |
| `jxcl-determinism` | `jxcl-machine`, `jxcl-memory` | *(none)* |
| `jxcl-fuzz` | `jxcl-decoding`, `jxcl-instructions` | *(none)* |
| `jxcl-golden` | `jxcl-simulator`, `jxcl-assembler` | `serde` |
| `jxcl-profiler` | `jxcl-trace`, `jxcl-cycle-model`, `jxcl-cache-model` | *(none)* |
| `jxcl-replay` | `jxcl-trace`, `jxcl-machine`, `jxcl-determinism` | *(none)* |
| `jxcl-simulator` | `jxcl-loader`, `jxcl-machine`, `jxcl-execution`, `jxcl-memory-map` | *(none)* |
| `jxcl-trace` | `jxcl-machine`, `jxcl-instructions` | *(none)* |

## Hardware/RTL (8)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-hardware` | `jxcl-rtl`, `jxcl-verilog`, `jxcl-vhdl`, `jxcl-netlist`, `jxcl-synthesis` | *(none)* |
| `jxcl-hardware-test` | `jxcl-hardware`, `jxcl-isa-schema` | *(none)* |
| `jxcl-hdl` | `jxcl-types` | *(none)* |
| `jxcl-netlist` | `jxcl-hdl` | *(none)* |
| `jxcl-rtl` | `jxcl-hdl`, `jxcl-opcodes`, `jxcl-constants`, `jxcl-alu` | *(none)* |
| `jxcl-synthesis` | `jxcl-hdl`, `jxcl-netlist` | *(none)* |
| `jxcl-verilog` | `jxcl-hdl` | *(none)* |
| `jxcl-vhdl` | `jxcl-hdl` | *(none)* |

## Cryptography (10)

| Crate | Depends on | External deps |
|---|---|---|
| `pq-aead` | *(none)* | `aes-gcm` |
| `pq-attestation` | `pq-envelope`, `pq-proof-types` | *(none)* |
| `pq-crypto` | `pq-kem`, `pq-kdf`, `pq-aead`, `pq-envelope`, `pq-keyring`, `pq-rotation` | *(none)* |
| `pq-envelope` | `pq-kem`, `pq-kdf`, `pq-aead` | *(none)* |
| `pq-kdf` | *(none)* | `hkdf`, `sha2` |
| `pq-kem` | *(none)* | `ml-kem` |
| `pq-keyring` | `pq-envelope` | *(none)* |
| `pq-policy` | `jxcl-errors` | *(none)* |
| `pq-rotation` | `pq-keyring` | *(none)* |
| `pq-signature` | *(none)* | `ml-dsa` |

## Storage/Data (7)

| Crate | Depends on | External deps |
|---|---|---|
| `pq-cache` | `pq-storage`, `pq-envelope`, `pq-keyring` | `redis` |
| `pq-journal` | `jxcl-bytes` | *(none)* |
| `pq-ledger` | `pq-journal`, `pq-proof-types` | *(none)* |
| `pq-migration` | *(none)* | `tiberius` |
| `pq-object-store` | `pq-storage`, `pq-envelope` | *(none)* |
| `pq-sql-vault` | `pq-storage`, `pq-envelope`, `pq-keyring`, `pq-migration` | `tiberius` |
| `pq-storage` | `pq-envelope` | *(none)* |

## Zero-Knowledge/Proof (5)

| Crate | Depends on | External deps |
|---|---|---|
| `pq-error-proof` | `pq-proof-types` | `ark-bls12-381`, `ark-ed-on-bls12-381`, `ark-groth16`, `ark-r1cs-std`, `ark-relations`, `ark-serialize`, `ark-snark`, `ark-std`, `sha2` |
| `pq-proof-bench` | `pq-proof-registry`, `pq-error-proof` | `criterion` |
| `pq-proof-registry` | `pq-proof-types` | *(none)* |
| `pq-proof-types` | *(none)* | *(none)* |
| `pq-proof-verifier` | `pq-proof-types`, `pq-proof-registry` | *(none)* |

## Network/Service (7)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-http` | `jxcl-errors` | `axum`, `reqwest` |
| `jxcl-network` | `jxcl-errors` | *(none)* |
| `jxcl-protocol` | `jxcl-simulator`, `jxcl-trace` | `serde` |
| `jxcl-rpc` | `jxcl-protocol`, `jxcl-network` | `tokio` |
| `jxcl-runtime` | `jxcl-config` | `tokio` |
| `jxcl-service` | `jxcl-http`, `jxcl-logging`, `jxcl-config` | *(none)* |
| `photo-cache-service` | `jxcl-service`, `jxcl-http`, `jxcl-network`, `pq-cache`, `pq-policy`, `pq-crypto` | `axum`, `reqwest`, `tokio` |

## Security/Observability/Integration (5)

| Crate | Depends on | External deps |
|---|---|---|
| `jxcl-audit` | `jxcl-logging` | *(none)* |
| `jxcl-conformance` | `jxcl-isa-schema`, `jxcl-hardware`, `jxcl-golden` | *(none)* |
| `jxcl-integration` | `jxcl-assembler`, `jxcl-simulator`, `pq-cache`, `pq-error-proof` | *(none)* |
| `jxcl-observability` | `jxcl-logging` | *(none)* |
| `jxcl-security` | `jxcl-errors` | *(none)* |

## Layering (high level)

```
                  applications
            (jxcl-cli, photo-cache-service)
                        |
                     services
        (jxcl-service, jxcl-rpc, jxcl-integration)
                        |
              integration / runtime
     (jxcl-simulator, jxcl-protocol, jxcl-runtime)
                        |
     crypto / storage / proof / network primitives
 (pq-*, jxcl-network, jxcl-http -- isolated from ISA internals)
                        |
              ISA / execution kernel
  (jxcl-execution, jxcl-memory, jxcl-assembler, jxcl-hardware, ...)
                        |
                    primitives
      (jxcl-types, jxcl-errors, jxcl-bitops, jxcl-endian, ...)
```

Enforced mechanically: no crate in `foundation`, `isa`, `execution`,
`memory`, `toolchain`, `debug`, or `hardware` may depend on a crate in
`storage` or `network` (`tools/check_workspace.py`'s
`check_forbidden_edges`). Crypto/proof crates depend only on
`std` + audited crates.io crates, never on service-layer crates.
