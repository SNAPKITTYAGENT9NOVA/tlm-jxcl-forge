# Crate Architecture: 6 → 100

This is the narrative companion to `docs/crates.toml` (the
machine-readable registry every crate is generated and implemented
from) and `docs/DEPENDENCY_GRAPH.md` (the DAG). Read `docs/BASELINE.md`
first for the pre-expansion state this decomposition is grounded in.

## Why 100, and why each one is real

The mandate was explicit: 100 crates, none of them fake. For a
workspace whose entire pre-expansion codebase is ~7,100 lines, that
mandate can only be honestly met one of two ways: pad the count with
thin wrapper crates (the outcome the mandate itself forbids), or build
enough genuinely new, real functionality that 100 crates each have a
real invariant to own. This expansion does the latter. Every entry in
`docs/crates.toml` carries a `source` field:

- **`extraction`** (37 crates): real code moved out of one of the
  existing six crates' existing modules, verbatim or near-verbatim,
  into its own crate. Traceable 1:1 to a specific pre-expansion file
  (see `docs/BASELINE.md`'s module map).
- **`new`** (60 crates): genuinely new functionality built for this
  expansion -- an object-file format and linker, a page table, a
  branch-target unit, an interrupt controller, a hash-to-curve-free
  Pedersen-commitment-free... no, concretely: things like a real
  relocatable object format, RTL codegen driven directly by the
  existing opcode table, ML-DSA signatures, a storage abstraction
  trait, a proof-scheme registry, a minimal RPC protocol, an audit
  ledger. Each is scoped to be genuinely testable and genuinely used by
  at least one other crate -- see each entry's `dependencies` field for
  who actually consumes it.
- **`facade`** (3 crates): `jxcl`, `pq-crypto`, and `pq-error-proof`
  keep their original names and public APIs, becoming thin re-export
  layers over the crates they were split into. This is what makes rule
  #1 ("preserve the existing six") true in the same commit that splits
  them apart -- nothing outside the workspace that depended on
  `jxcl::isa::opcodes` or `pq_crypto::KeyRing` needs to change.

Where a proposed split failed the "does this crate own a real
invariant / can it evolve independently" test (rule #6), it was merged
into a neighboring crate instead of forced into existence. The
`pq-crypto` split, for instance, does *not* produce a separate crate
for "the nonce" or "the ciphertext" -- those are fields of
`pq-envelope::Envelope`, not independent responsibilities.

## Reading the registry by category

| Category | Count | Crates 9-range | Built on |
|---|---|---|---|
| Foundation | 8 | jxcl-types … jxcl-config | std only |
| ISA | 12 | jxcl … jxcl-isa-schema | Foundation |
| Execution | 10 | jxcl-alu … jxcl-cycle-model | ISA, Foundation |
| Memory | 8 | jxcl-memory … jxcl-heap | Foundation |
| Toolchain | 12 | jxcl-binary … jxcl-cli | ISA, Memory, Execution |
| Debug/Simulation | 8 | jxcl-debugger … jxcl-fuzz | Execution, Memory, Toolchain |
| Hardware/RTL | 8 | jxcl-hardware … jxcl-hardware-test | ISA (opcodes/constants), Execution (ALU) |
| Cryptography | 10 | pq-crypto … pq-policy | std + audited crates.io only |
| Storage/Data | 7 | pq-cache … pq-migration | Cryptography |
| Zero-Knowledge/Proof | 5 | pq-error-proof … pq-proof-bench | std + arkworks (isolated) |
| Network/Service | 7 | jxcl-network … photo-cache-service | Debug/Simulation (jxcl-simulator), Storage, Cryptography |
| Security/Observability/Integration | 5 | jxcl-security … jxcl-conformance | cross-cutting, depends down into every layer it audits |

## The honesty boundary: Hardware/RTL

This environment has no `iverilog`, `verilator`, or `yosys` (checked
and recorded in `docs/BASELINE.md`). The Hardware/RTL crates are real
code -- a real HDL AST, real Verilog/VHDL text-rendering backends, a
real (if intentionally toy) structural synthesis pass, and a real
mechanical cross-check that the generated decoder's case arms match
`jxcl-opcodes`' actual table -- but they cannot be validated by
simulating the generated RTL against real hardware-simulation
semantics. See `docs/HARDWARE_LIMITATIONS.md` for exactly what is and
isn't verified, so nobody downstream mistakes "compiles and matches a
golden file" for "was simulated and behaves like real silicon."

## Ownership boundaries (no overlaps)

Per rule #6, every crate answers these seven questions in
`docs/crates.toml` (as `owns`, `public_api`, `dependencies`, `tests`):
what invariant it owns, what API it exposes, what data types it owns,
what it depends on, what depends on it, whether it's independently
testable, and whether it can evolve independently. A few boundaries
that could plausibly have been merged, and why they weren't:

- `pq-keyring` (the ring data structure) vs. `pq-rotation` (the
  lifecycle policy for when entries move between Active/DecryptOnly/
  Retired): the data structure doesn't care about policy, and a future
  alternate rotation policy (e.g. time-based auto-retirement) should be
  implementable without touching the ring itself.
- `jxcl-binary` (the final linked container format) vs. `jxcl-object`
  (the relocatable pre-link format): these are genuinely different
  wire formats with different consumers (the loader vs. the linker).
- `jxcl-logging` (subscriber init) vs. `jxcl-observability` (span/metric
  *conventions* built on top) vs. `jxcl-audit` (a structured event
  *schema* for security-relevant events): three different callers
  (anything that starts up; anything serving requests; anything making
  a security-relevant decision) with three different reasons to change.
