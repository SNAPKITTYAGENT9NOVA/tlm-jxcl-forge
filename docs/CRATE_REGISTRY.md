# Crate Registry (human-readable index)

Generated view of `docs/crates.toml`, the authoritative machine-readable
registry. Regenerate with `python3 tools/gen_crate_registry_doc.py`.

**Total: 100 crates.**

## Foundation (8)

### `jxcl-bitops`  _extraction:jxcl/src/isa/opcodes.rs,jxcl/src/encoding/*_
Bitfield extraction/insertion and sign-extension helpers used by encoding, decoding, and the ALU.
- **Owns:** extract_bits/insert_bits/sign_extend and related bit-level primitives.
- **Public API:** extract_bits, insert_bits, sign_extend
- **Tests:** unit, property

### `jxcl-bytes`  _new_
A bounds-checked byte-buffer cursor used by the encoder, decoder, binary container, and object format.
- **Owns:** The ByteCursor read/write-with-bounds-checking API.
- **Public API:** ByteCursor
- **Tests:** unit, boundary

### `jxcl-config`  _extraction:photo-cache-service/src/lib.rs,pq-crypto/src/lib.rs_
Environment-variable configuration parsing helpers (typed env_or, hex-seed decoding) shared by the CLI and services.
- **Owns:** env_or/env_or_parse/decode_hex_seed and the 'log the resolved value only if not secret' convention.
- **Public API:** env_or, env_or_parse, decode_hex_seed
- **Tests:** unit, boundary

### `jxcl-constants`  _extraction:jxcl/src/isa/constants.rs_
Architecture-wide constants: word width, register count, memory size, opcode width.
- **Owns:** The single authoritative set of ISA width/count constants.
- **Public API:** WORD_BITS, REGISTER_COUNT, MEMORY_SIZE, OPCODE_BITS
- **Tests:** unit

### `jxcl-endian`  _extraction:jxcl/src/encoding/*_
The ISA's fixed-endianness integer <-> byte conversions, isolated from the general byte-buffer cursor.
- **Owns:** to_arch_bytes/from_arch_bytes for each integer width.
- **Public API:** to_arch_bytes, from_arch_bytes
- **Tests:** unit, property

### `jxcl-errors`  _extraction:jxcl/src/errors.rs_
The crate-wide error type shared across the ISA/execution/toolchain crates.
- **Owns:** The Error enum and its Display/std::error::Error impls.
- **Public API:** Error
- **Tests:** unit

### `jxcl-logging`  _extraction:jxcl/src/main.rs,photo-cache-service/src/lib.rs_
Shared tracing/logging initialization used by the CLI and the network services.
- **Owns:** init_logging() and the RUST_LOG-driven subscriber setup convention.
- **Public API:** init_logging
- **Tests:** unit

### `jxcl-types`  _new_
Shared newtype wrappers for architectural primitive values (word, address, register index, immediate) so every ISA/execution crate agrees on one representation.
- **Owns:** The Word/Address/RegisterIndex/Immediate newtypes and their arithmetic/conversion impls.
- **Public API:** Word, Address, RegisterIndex, Immediate
- **Tests:** unit

## ISA (12)

### `jxcl`  _facade_
Backward-compatible facade preserving the original crate's public API and `jxcl` binary name over the newly split crates.
- **Owns:** Nothing new -- re-exports the ISA/execution/toolchain crates under their original module paths (isa::, encoding::, alu::, memory::, machine::, execution::, control::, binary::, validator::, assembler::, disassembler::, debugger::).
- **Public API:** (re-exports of the pre-expansion public API, unchanged)
- **Tests:** integration

### `jxcl-decoding`  _extraction:jxcl/src/encoding/decoder.rs_
Decodes an Instruction from its binary wire format -- the exact inverse of jxcl-encoding.
- **Owns:** decode(bytes) -> Instruction; the only place instruction bit layout is read.
- **Public API:** decode
- **Tests:** unit, property, golden, fuzz

### `jxcl-encoding`  _extraction:jxcl/src/encoding/encoder.rs_
Encodes an Instruction to its binary wire format.
- **Owns:** encode(Instruction) -> bytes; the only place instruction bit layout is written.
- **Public API:** encode
- **Tests:** unit, property, golden

### `jxcl-flags`  _extraction:jxcl/src/isa/flags.rs_
The flags register: zero/carry/overflow/negative/interrupt-mask bit semantics.
- **Owns:** The Flags type and its bit-level get/set semantics.
- **Public API:** Flags
- **Tests:** unit, property

### `jxcl-instruction-validation`  _new_
Per-instruction operand-legality checks (register range, immediate range, addressing-mode compatibility with the opcode) shared by the decoder and the static binary validator.
- **Owns:** validate_instruction(&Instruction) -> Result<(), Error> -- the single source of truth for 'is this a legal instruction', so the decoder and jxcl-validator never diverge.
- **Public API:** validate_instruction
- **Tests:** unit, boundary, error-path

### `jxcl-instructions`  _extraction:jxcl/src/isa/instruction.rs_
The Instruction type tying an opcode to its operands -- the canonical in-memory instruction representation.
- **Owns:** The Instruction struct.
- **Public API:** Instruction
- **Tests:** unit

### `jxcl-isa-metadata`  _new_
Queryable descriptive metadata about the running ISA build (opcode count, register count, build flags) used by `jxcl inspect` and the debugger.
- **Owns:** IsaMetadata::describe().
- **Public API:** IsaMetadata, describe
- **Tests:** unit

### `jxcl-isa-schema`  _new_
A serde-serializable schema of the ISA generated from jxcl-opcodes/jxcl-constants/jxcl-registers/jxcl-flags, plus a mechanical cross-check against the numbers documented in docs/ISA_SPEC.md.
- **Owns:** IsaSchema and the spec-vs-code conformance check that ISA_SPEC.md's stated widths/opcode-count match the generated schema.
- **Public API:** IsaSchema, IsaSchema::generate, IsaSchema::check_against_spec
- **Tests:** unit, conformance

### `jxcl-isa-versioning`  _new_
ISA/binary-format version negotiation so old binaries fail closed against an incompatible newer decoder rather than silently misdecoding.
- **Owns:** IsaVersion, its embedding in the binary container header, and the compatibility check.
- **Public API:** IsaVersion, IsaVersion::is_compatible_with
- **Tests:** unit, boundary

### `jxcl-opcodes`  _extraction:jxcl/src/isa/opcodes.rs_
The single authoritative opcode registry: ids, mnemonics, operand-shape metadata.
- **Owns:** The opcode table -- no other crate may define or duplicate opcode identifiers.
- **Public API:** Opcode, OPCODE_TABLE, opcode_by_mnemonic
- **Tests:** unit, golden

### `jxcl-operands`  _extraction:jxcl/src/isa/operand.rs_
The operand/addressing-mode model (register-direct, immediate, memory-indirect, etc.).
- **Owns:** The Operand enum and addressing-mode variants.
- **Public API:** Operand, AddressingMode
- **Tests:** unit

### `jxcl-registers`  _extraction:jxcl/src/isa/registers.rs_
The register file model: indices, names, widths.
- **Owns:** The Register enum/index type and register metadata.
- **Public API:** Register, RegisterFile
- **Tests:** unit

## Execution (10)

### `jxcl-alu`  _extraction:jxcl/src/alu.rs_
Arithmetic/logic unit semantics: add/sub/mul/div/shift/bitwise, with flag updates.
- **Owns:** All arithmetic semantics -- no other crate performs ALU-equivalent computation independently.
- **Public API:** Alu, Alu::execute
- **Tests:** unit, property, boundary

### `jxcl-branch`  _new_
Branch-target address computation (relative/absolute addressing) split out of condition evaluation.
- **Owns:** compute_branch_target.
- **Public API:** compute_branch_target
- **Tests:** unit, boundary

### `jxcl-control`  _extraction:jxcl/src/control.rs_
Branch/jump condition evaluation against the flags register.
- **Owns:** Condition-code evaluation (branch-taken decisions).
- **Public API:** evaluate_condition
- **Tests:** unit

### `jxcl-cycle-model`  _new_
A per-opcode cycle-cost table and accumulator giving a deterministic total-cycle count for a run.
- **Owns:** CYCLE_COST_TABLE and CycleCounter.
- **Public API:** CycleCounter, cycle_cost
- **Tests:** unit, determinism

### `jxcl-dispatch`  _new_
Table-driven opcode-to-handler dispatch, extracted from the fetch/decode/execute loop's match statement so dispatch can be tested and extended independently of execution semantics.
- **Owns:** The dispatch table (Opcode -> handler fn pointer).
- **Public API:** DispatchTable, DispatchTable::lookup
- **Tests:** unit

### `jxcl-exceptions`  _new_
The machine's exception model: illegal opcode, misaligned access, division by zero, and the trap-handling hook.
- **Owns:** The Exception enum and the trap dispatch contract.
- **Public API:** Exception, TrapHandler
- **Tests:** unit, error-path

### `jxcl-execution`  _extraction:jxcl/src/execution.rs_
The fetch/decode/execute loop: owns instruction execution, wiring together decoding, dispatch, the ALU, memory, and exceptions.
- **Owns:** step(&mut Machine) -- the only place an instruction is actually executed.
- **Public API:** step, run
- **Tests:** unit, property, golden

### `jxcl-interrupts`  _new_
A deterministic external-interrupt injection mechanism (priority queue + mask flag) for embedding jxcl in a simulator or RPC host.
- **Owns:** InterruptController: queue/mask/deliver.
- **Public API:** InterruptController
- **Tests:** unit, determinism

### `jxcl-machine`  _extraction:jxcl/src/machine.rs_
Architectural state: the register file, flags, and program counter as one cohesive Machine struct.
- **Owns:** The Machine struct -- the single authoritative representation of architectural state.
- **Public API:** Machine
- **Tests:** unit

### `jxcl-pipeline`  _new_
A staged fetch/decode/execute/writeback pipeline-stage model used to report realistic per-instruction cycle costs, distinct from the (non-pipelined) reference execution semantics.
- **Owns:** PipelineStage and the stage-advance state machine.
- **Public API:** PipelineStage, Pipeline
- **Tests:** unit

## Memory (8)

### `jxcl-address-space`  _new_
Named memory regions (code/data/stack) with permission flags layered over raw storage.
- **Owns:** AddressSpace and Region (base, len, permissions).
- **Public API:** AddressSpace, Region, Permission
- **Tests:** unit, boundary

### `jxcl-cache-model`  _new_
A direct-mapped/set-associative cache simulation layered over memory, for profiling hit/miss behavior.
- **Owns:** CacheModel and its hit/miss accounting.
- **Public API:** CacheModel
- **Tests:** unit, determinism

### `jxcl-heap`  _new_
A simple bump/free-list allocator operating within an address space, for programs needing dynamic memory.
- **Owns:** Heap::alloc/free.
- **Public API:** Heap
- **Tests:** unit, error-path

### `jxcl-load-store`  _new_
Typed, sign-extension-aware load/store helpers (u8/u16/u32/u64 and signed variants) between raw memory and the execution engine.
- **Owns:** load_u8/16/32/64, store_u8/16/32/64 and their signed counterparts.
- **Public API:** load_u8, load_u16, load_u32, load_u64, store_u8, store_u16, store_u32, store_u64
- **Tests:** unit, property

### `jxcl-memory`  _extraction:jxcl/src/memory.rs_
Raw byte-addressable memory storage and bounds-checked byte-level access.
- **Owns:** The Memory struct -- the only place raw bytes are stored.
- **Public API:** Memory
- **Tests:** unit, property, boundary

### `jxcl-memory-map`  _new_
The concrete default memory layout (where code/stack/heap/MMIO live) consumed by the loader and machine setup.
- **Owns:** MemoryMap, the default layout constant.
- **Public API:** MemoryMap, DEFAULT_MEMORY_MAP
- **Tests:** unit

### `jxcl-page-table`  _new_
A single-level virtual-to-physical page table with map/unmap/translate and page-fault on unmapped access.
- **Owns:** PageTable.
- **Public API:** PageTable
- **Tests:** unit, error-path

### `jxcl-stack`  _new_
Typed stack-pointer push/pop with overflow/underflow checking for call/return semantics.
- **Owns:** Stack::push/pop and its bounds checks.
- **Public API:** Stack
- **Tests:** unit, boundary

## Binary/Toolchain (12)

### `jxcl-assembler`  _extraction:jxcl/src/assembler/mod.rs_
The two-pass assembler: source text to an object file (plus a convenience path assembling and linking a single file straight to a binary, preserving the original single-step CLI workflow).
- **Owns:** assemble(source) -> ObjectFile.
- **Public API:** assemble, assemble_to_binary
- **Tests:** unit, integration, golden

### `jxcl-binary`  _extraction:jxcl/src/binary.rs_
The final linked-executable container format: header, version, code/data sections.
- **Owns:** The binary container's byte layout.
- **Public API:** BinaryContainer
- **Tests:** unit, boundary, golden

### `jxcl-cli`  _extraction:jxcl/src/main.rs_
The `jxcl` command-line interface: asm/disasm/run/inspect/validate subcommands.
- **Owns:** The CLI argument parsing and subcommand dispatch (main.rs's logic).
- **Public API:** main
- **Tests:** integration

### `jxcl-debug-info`  _new_
A simple address-to-source-line debug info format emitted by the assembler and consumed by the disassembler/debugger.
- **Owns:** LineTable.
- **Public API:** LineTable
- **Tests:** unit

### `jxcl-disassembler`  _extraction:jxcl/src/disassembler.rs_
Disassembles a binary/object file back to readable assembly text, symbolizing addresses when debug info is present.
- **Owns:** disassemble(bytes) -> String.
- **Public API:** disassemble
- **Tests:** unit, golden

### `jxcl-lexer`  _extraction:jxcl/src/assembler/lexer.rs_
Tokenizes assembly source text.
- **Owns:** The Token type and lexer.
- **Public API:** lex, Token
- **Tests:** unit, boundary

### `jxcl-linker`  _new_
Resolves symbols and applies relocations across one or more object files into a single linked binary container.
- **Owns:** link(&[ObjectFile]) -> BinaryContainer.
- **Public API:** link
- **Tests:** unit, integration, error-path

### `jxcl-loader`  _new_
Loads a linked binary container into a memory image laid out per jxcl-memory-map, ready to run.
- **Owns:** load(BinaryContainer) -> (Memory, entry_point).
- **Public API:** load
- **Tests:** unit, integration

### `jxcl-object`  _new_
A relocatable object-file format (sections + symbol table + relocation table) distinct from the final linked binary container.
- **Owns:** ObjectFile serialization/deserialization.
- **Public API:** ObjectFile
- **Tests:** unit, boundary

### `jxcl-parser`  _extraction:jxcl/src/assembler/parser.rs_
Parses a token stream into an assembly AST (instructions, labels, directives).
- **Owns:** The assembly AST and its parser.
- **Public API:** parse, AstNode
- **Tests:** unit, error-path

### `jxcl-relocations`  _new_
Relocation entry types and the patch-in-place logic that fixes up an encoded instruction once a symbol's final address is known.
- **Owns:** Relocation and apply_relocation.
- **Public API:** Relocation, apply_relocation
- **Tests:** unit

### `jxcl-symbols`  _new_
The symbol table type (name -> address/section) shared by the assembler, object format, linker, and disassembler.
- **Owns:** SymbolTable.
- **Public API:** SymbolTable, Symbol
- **Tests:** unit

## Debug/Simulation (8)

### `jxcl-debugger`  _extraction:jxcl/src/debugger.rs_
Interactive/trace-mode debugger: step, breakpoint, and inspect a running machine.
- **Owns:** The Debugger driver.
- **Public API:** Debugger
- **Tests:** unit, integration

### `jxcl-determinism`  _extraction:jxcl/tests/property_tests.rs_
Machine state snapshot/restore and the determinism property-test harness.
- **Owns:** Snapshot and the Snapshot/Restore trait.
- **Public API:** Snapshot, SnapshotRestore
- **Tests:** unit, property, determinism

### `jxcl-fuzz`  _extraction:jxcl/tests/fuzz_decoder.rs_
A structured fuzz harness for the decoder (and other parser/decoder boundaries) against malformed input.
- **Owns:** The fuzz target(s).
- **Public API:** fuzz_decode_target
- **Tests:** fuzz

### `jxcl-golden`  _extraction:jxcl/tests/golden_vectors.rs_
The golden-vector format, loader, and canonical vector files used as the ISA conformance baseline.
- **Owns:** GoldenVector and tests/vectors/.
- **Public API:** GoldenVector, load_vectors, run_vector
- **Tests:** golden, conformance

### `jxcl-profiler`  _new_
Aggregates a trace, the cycle model, and the cache model into run statistics (instruction count, cycles, cache hit rate).
- **Owns:** ProfileReport.
- **Public API:** ProfileReport, profile_run
- **Tests:** unit, determinism

### `jxcl-replay`  _new_
Deterministically replays a recorded trace to reconstruct machine state at any step, without re-running the original inputs.
- **Owns:** Replay::seek_to_step.
- **Public API:** Replay
- **Tests:** unit, determinism

### `jxcl-simulator`  _new_
An embeddable top-level simulation API (load + run in one call) for using jxcl as a library from other Rust programs/services.
- **Owns:** Simulator::new/run.
- **Public API:** Simulator, RunResult
- **Tests:** unit, integration

### `jxcl-trace`  _new_
The per-step execution trace record format and its writer/reader, reused by the profiler and replay.
- **Owns:** TraceStep and TraceLog.
- **Public API:** TraceStep, TraceLog
- **Tests:** unit, serialization

## Hardware/RTL (8)

### `jxcl-hardware`  _new_
Top-level facade integrating the HDL AST, both text backends, and the toy synthesis pass behind one emit API.
- **Owns:** emit_verilog/emit_vhdl/emit_netlist entry points for the jxcl core datapath.
- **Public API:** emit_verilog, emit_vhdl, emit_netlist
- **Tests:** integration

### `jxcl-hardware-test`  _new_
Golden-file regression tests for generated Verilog/VHDL/netlist, plus the mechanical width/opcode-count cross-check against the ISA schema.
- **Owns:** The hardware golden files and the ISA-vs-RTL conformance check.
- **Public API:** (test-only crate)
- **Tests:** golden, conformance

### `jxcl-hdl`  _new_
A backend-agnostic hardware-description intermediate representation: modules, ports, signals, always-blocks, case-statements.
- **Owns:** The HDL AST types.
- **Public API:** Module, Port, Signal, Statement
- **Tests:** unit

### `jxcl-netlist`  _new_
A structural gate-level netlist intermediate representation (primitive gates + wires).
- **Owns:** The Netlist/Gate/Wire types.
- **Public API:** Netlist, Gate, Wire
- **Tests:** unit

### `jxcl-rtl`  _new_
The RTL description of jxcl's core datapath (opcode decoder, ALU, register file) generated as a jxcl-hdl AST directly from jxcl-opcodes/jxcl-constants/jxcl-alu -- the mechanically-verifiable software/hardware bridge required by docs/RTL_CONTRACT.md.
- **Owns:** build_decoder_module/build_alu_module/build_register_file_module.
- **Public API:** build_decoder_module, build_alu_module, build_register_file_module
- **Tests:** unit, conformance

### `jxcl-synthesis`  _new_
A toy (explicitly documented, non-production) structural synthesis pass lowering a subset of the HDL AST into primitive-gate netlist form.
- **Owns:** synthesize(Module) -> Netlist.
- **Public API:** synthesize
- **Tests:** unit

### `jxcl-verilog`  _new_
Renders a jxcl-hdl AST to synthesizable Verilog-2001 text.
- **Owns:** The Verilog text backend.
- **Public API:** render_verilog
- **Tests:** unit, golden

### `jxcl-vhdl`  _new_
Renders the same jxcl-hdl AST to VHDL text, proving the AST is backend-agnostic.
- **Owns:** The VHDL text backend.
- **Public API:** render_vhdl
- **Tests:** unit, golden

## Cryptography (10)

### `pq-aead`  _extraction:pq-crypto/src/lib.rs_
AES-256-GCM authenticated encryption/decryption of the plaintext under the derived key.
- **Owns:** aead_encrypt/aead_decrypt.
- **Public API:** aead_encrypt, aead_decrypt
- **Tests:** unit, tamper, wrong-key

### `pq-attestation`  _new_
Combines sealing a value (pq-envelope) with a verifiable attestation (pq-proof-types) that it was sealed under a specific, named key version, in one call.
- **Owns:** seal_with_attestation / open_with_attestation.
- **Public API:** seal_with_attestation, open_with_attestation
- **Tests:** unit, integration

### `pq-crypto`  _facade_
Backward-compatible facade preserving the original crate's public API over the newly split kem/kdf/aead/envelope/keyring/rotation crates.
- **Owns:** Nothing new -- re-exports seal/open/KeyPair/Envelope/KeyRing/KeyStatus/Error under their original paths.
- **Public API:** (re-exports of the pre-expansion public API, unchanged)
- **Tests:** integration

### `pq-envelope`  _extraction:pq-crypto/src/lib.rs_
The sealed-value wire format: key version, KEM ciphertext, nonce, AEAD ciphertext.
- **Owns:** Envelope and its to_bytes/from_bytes wire format -- the only place the envelope byte layout is defined.
- **Public API:** Envelope, seal, open
- **Tests:** unit, boundary, known-answer

### `pq-kdf`  _extraction:pq-crypto/src/lib.rs_
HKDF-SHA256 expansion of a KEM shared secret into an AES-256 key, with fixed domain separation.
- **Owns:** derive_aes_key.
- **Public API:** derive_aes_key
- **Tests:** unit, known-answer

### `pq-kem`  _extraction:pq-crypto/src/lib.rs_
ML-KEM-768 (NIST FIPS 203) key generation and encapsulation/decapsulation.
- **Owns:** KeyPair and the raw KEM step.
- **Public API:** KeyPair, EncapsulationKey, DecapsulationKey
- **Tests:** unit, known-answer

### `pq-keyring`  _extraction:pq-crypto/src/lib.rs_
The KeyRing data structure: an indexed set of key-pair entries.
- **Owns:** KeyRing's storage and lookup (insert/get/iterate) -- not rotation policy, see pq-rotation.
- **Public API:** KeyRing
- **Tests:** unit

### `pq-policy`  _new_
Consolidates scattered policy decisions (TLS-required-in-production, minimum seed/key length) into one crate with a real Policy trait, replacing duplicated ad hoc checks in photo-cache-service and pq-crypto.
- **Owns:** The Policy trait and the concrete TlsRequiredInProduction/MinimumSeedLength policies.
- **Public API:** Policy, TlsRequiredInProduction, MinimumSeedLength
- **Tests:** unit, boundary

### `pq-rotation`  _extraction:pq-crypto/src/lib.rs_
Key-rotation policy: the Active/DecryptOnly/Retired lifecycle and the set_active/retire transition rules, kept separate from the KeyRing data structure itself.
- **Owns:** KeyStatus and the rotation state-transition rules.
- **Public API:** KeyStatus, set_active, retire
- **Tests:** unit, rotation

### `pq-signature`  _new_
ML-DSA (NIST FIPS 204 / Dilithium) signing and verification -- a second, complementary post-quantum primitive (authenticity, alongside pq-kem's confidentiality).
- **Owns:** SigningKey/VerifyingKey and sign/verify.
- **Public API:** SigningKey, VerifyingKey, sign, verify
- **Tests:** unit, known-answer, tamper, wrong-key

## Storage/Data (7)

### `pq-cache`  _extraction:pq-cache/src/lib.rs_
A Redis-backed cache whose entries are sealed with pq-crypto before they reach Redis (existing public API preserved; now also implements pq-storage::SealedStore).
- **Owns:** The Redis-specific connection/get/set_with_ttl implementation.
- **Public API:** EncryptedCache, SealedStore for EncryptedCache
- **Tests:** unit, integration

### `pq-journal`  _new_
An append-only, length-prefixed write-ahead log file format with replay, for durability.
- **Owns:** Journal::append/replay.
- **Public API:** Journal
- **Tests:** unit, boundary

### `pq-ledger`  _new_
A tamper-evident, hash-chained audit ledger recording proof attestations in order, built on pq-journal.
- **Owns:** Ledger::record/verify_chain.
- **Public API:** Ledger
- **Tests:** unit, tamper

### `pq-migration`  _new_
Applies pq-sql-vault's sql/*.sql migration files in order against a live connection and tracks which have been applied.
- **Owns:** run_migrations.
- **Public API:** run_migrations, Migration
- **Tests:** unit

### `pq-object-store`  _new_
Chunked/streamed large-blob storage built on top of any SealedStore, splitting values above a size threshold into sealed chunks with a manifest.
- **Owns:** put_object/get_object and the chunk-manifest format.
- **Public API:** put_object, get_object
- **Tests:** unit, integration

### `pq-sql-vault`  _extraction:pq-sql-vault/src/lib.rs_
A SQL Server-backed store for sealed values (existing public API preserved; now implements pq-storage::SealedStore and applies its schema via pq-migration).
- **Owns:** The SQL Server-specific connection/get/set_with_ttl implementation and stored-procedure calls.
- **Public API:** SqlVault, SealedStore for SqlVault
- **Tests:** unit, integration (ignored, no live SQL Server)

### `pq-storage`  _new_
The SealedStore trait both pq-cache and pq-sql-vault implement, letting callers be storage-agnostic.
- **Owns:** The SealedStore trait.
- **Public API:** SealedStore
- **Tests:** unit

## Zero-Knowledge/Proof (5)

### `pq-error-proof`  _extraction:pq-error-proof/src/lib.rs_
The concrete Groth16/arkworks proof-of-commitment-opening implementation, registered into pq-proof-registry as a ProofScheme (existing public API preserved).
- **Owns:** The Groth16 circuit and its Params/Attestation types.
- **Public API:** Params, Attestation, attest, verify
- **Tests:** unit, tamper, cross-setup, serialization, unlinkability

### `pq-proof-bench`  _new_
Criterion benchmarks measuring attest()/verify() throughput for registered proof schemes.
- **Owns:** The benchmark harness (dev-only, not a library dependency of anything).
- **Public API:** (bench-only crate)
- **Tests:** benchmark

### `pq-proof-registry`  _new_
A registry mapping scheme-id strings to boxed ProofScheme implementations, supporting future proof schemes beyond Groth16-Pedersen.
- **Owns:** ProofRegistry::register/get.
- **Public API:** ProofRegistry
- **Tests:** unit

### `pq-proof-types`  _new_
Backend-independent proof types: the ProofScheme trait and shared Attestation/Error types, so callers never depend on a concrete proof backend directly.
- **Owns:** The ProofScheme trait.
- **Public API:** ProofScheme, AttestationBytes, ProofError
- **Tests:** unit

### `pq-proof-verifier`  _new_
A verifier facade that looks up the right scheme via pq-proof-registry and verifies, so callers never touch arkworks types directly.
- **Owns:** verify_attestation(scheme_id, ...).
- **Public API:** verify_attestation
- **Tests:** unit, integration

## Network/Service (7)

### `jxcl-http`  _extraction:photo-cache-service/src/lib.rs_
Shared HTTP client/server helpers (timeout wrapper, error-to-status mapping) extracted from photo-cache-service's duplicated per-binary logic.
- **Owns:** with_timeout and AppError-to-http-status mapping.
- **Public API:** with_timeout, error_to_status
- **Tests:** unit

### `jxcl-network`  _extraction:photo-cache-service/src/lib.rs_
Low-level network utilities: connection-string credential redaction, address/port parsing.
- **Owns:** redact_url_credentials and address-parsing helpers.
- **Public API:** redact_url_credentials
- **Tests:** unit

### `jxcl-protocol`  _new_
A serde-serializable request/response protocol for remote jxcl-machine control: assemble, run, return trace/final state.
- **Owns:** the Request/Response wire types.
- **Public API:** Request, Response
- **Tests:** unit, serialization

### `jxcl-rpc`  _new_
A minimal RPC server/client implementing jxcl-protocol over line-delimited JSON on TCP.
- **Owns:** RpcServer/RpcClient.
- **Public API:** RpcServer, RpcClient
- **Tests:** unit, integration

### `jxcl-runtime`  _new_
Tokio runtime configuration/startup helpers (worker-thread count from env, panic-hook wiring).
- **Owns:** build_runtime.
- **Public API:** build_runtime
- **Tests:** unit

### `jxcl-service`  _extraction:photo-cache-service/src/bin/*.rs_
Generic service scaffolding (graceful shutdown, health-check endpoint pattern, startup logging) extracted from photo-cache-service's two binaries' shared boilerplate.
- **Owns:** serve_with_graceful_shutdown and the healthz handler pattern.
- **Public API:** serve_with_graceful_shutdown, healthz_handler
- **Tests:** unit, integration

### `photo-cache-service`  _extraction:photo-cache-service/src/*_
The two axum binaries (server, server-cached) mirroring the original Express demo, rebuilt on jxcl-service/jxcl-http/jxcl-network/pq-storage/pq-policy (existing public behavior preserved).
- **Owns:** The GET /photos and GET /healthz handlers and the two binaries.
- **Public API:** (binaries: server, server-cached)
- **Tests:** unit, integration (live Redis)

## Security/Observability/Integration (5)

### `jxcl-audit`  _new_
A structured audit-event schema and emission helper, distinct from jxcl-logging's generic initialization.
- **Owns:** AuditEvent and emit_audit_event.
- **Public API:** AuditEvent, emit_audit_event
- **Tests:** unit

### `jxcl-conformance`  _new_
The mechanical spec-vs-code conformance suite: checks docs/ISA_SPEC.md and docs/RTL_CONTRACT.md's stated facts against jxcl-isa-schema and jxcl-hardware's generated RTL.
- **Owns:** The final mechanical verification layer tying software ISA and hardware RTL to their documentation.
- **Public API:** (test-only crate)
- **Tests:** conformance

### `jxcl-integration`  _new_
An integration-test-only crate exercising the full stack end-to-end: assemble, run in jxcl-simulator, seal/store in pq-cache, attest with pq-error-proof.
- **Owns:** The canonical whole-workspace integration tests.
- **Public API:** (test-only crate)
- **Tests:** integration

### `jxcl-observability`  _new_
Metrics/span conventions (a RequestSpan helper, standard metric names) built atop jxcl-logging, consumed by jxcl-service.
- **Owns:** RequestSpan and the metric-naming convention.
- **Public API:** RequestSpan
- **Tests:** unit

### `jxcl-security`  _new_
Cross-cutting secret-redaction utilities, generalizing the two independent redaction implementations found in photo-cache-service (Redis URL) and pq-sql-vault (ADO connection string) into one shared, tested implementation.
- **Owns:** redact_secret_bearing_string and the shared redaction convention.
- **Public API:** redact_secret_bearing_string
- **Tests:** unit, boundary
