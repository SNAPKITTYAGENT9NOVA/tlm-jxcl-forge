# Crate Generation Plan

## Phase sequence

0. **Inventory** (done -- `docs/BASELINE.md`'s module map).
1. **Baseline** (done -- `docs/BASELINE.md`'s gate results).
2. **Architecture graph** (done -- `docs/crates.toml`, validated acyclic
   and edge-clean by `tools/check_workspace.py`).
3. **Mass scaffold**: `tools/scaffold_crates.py` reads `docs/crates.toml`
   and generates, for every crate not yet on disk, `Cargo.toml`,
   `src/lib.rs` (or `src/main.rs`), and `README.md`, then rewrites the
   root `Cargo.toml` workspace `members` list to match the registry
   exactly. Scaffolded crates compile (empty-but-valid modules with a
   `// TODO(batch): <name>` marker and a placeholder test) so
   `cargo check --workspace` is green immediately after scaffolding,
   before any real implementation lands.
4. **Parallel implementation** (see batch table below): each batch is
   handed to one subagent with (a) the frozen `docs/crates.toml` entries
   for its crates, (b) the exact pre-expansion source file(s) to extract
   real code from, per `docs/BASELINE.md`'s module map, and (c) the new
   functionality it must build for its "new"-sourced crates. Batches run
   concurrently because their public API contracts (the `public_api`
   field per crate) are frozen in step 2 and none may be redefined
   independently mid-implementation.
5. **Integration**: the coordinator (this session) resolves any
   cross-batch friction (e.g. a batch discovering it needs one more
   function from a crate outside its assignment), re-runs the full
   workspace build, and reconciles the facade crates (`jxcl`,
   `pq-crypto`, `pq-error-proof`) so the pre-expansion public API is
   provably unchanged.
6. **Workspace-wide gates**: fmt, check, test, clippy, `cargo audit`,
   `tools/check_workspace.py --disk` (crate count + DAG + forbidden
   edges + on-disk scaffolding-matches-registry), documentation
   completeness.
7. **Release candidate**: `docs/CRATE_CATALOG.md`, `docs/BUILD_MATRIX.md`
   finalized; commit; push; draft PR.

## Batches (implementation workstreams)

| Batch | Categories | Crate count | Depends on (must land first) |
|---|---|---|---|
| A | Foundation + ISA | 20 | *(none -- innermost layer)* |
| B | Execution + Memory | 18 | A |
| C | Toolchain | 12 | A, B |
| D | Debug/Simulation | 8 | A, B, C |
| E | Hardware/RTL | 8 | A (opcodes/constants), B (ALU) |
| F | Cryptography | 10 | *(none -- independent of ISA)* |
| G | Storage | 7 | F |
| H | Proof systems | 5 | *(none -- independent of ISA/crypto)* |
| I | Network/Services | 7 | D (jxcl-simulator), F, G |
| J | Security/Observability/Integration | 5 | everything it audits (I, H, E, D) |

Batches A-E and F-H are mutually independent and run fully in parallel.
I depends on D/F/G landing first (it composes them); J runs last since
`jxcl-integration`/`jxcl-conformance` exercise the whole stack.

## What "done" means per crate

Per rule #19, every crate must ship with: `Cargo.toml`, `src/lib.rs` (or
`src/main.rs`), `README.md` (purpose, architecture, public API,
dependencies, examples, testing, security/performance notes where
relevant), unit tests, and (per its `tests` field in `docs/crates.toml`)
whichever of property/golden/fuzz/determinism/serialization/conformance/
benchmark tests apply. A crate is not "implemented" in
`docs/crates.toml`'s `status` field until `cargo test -p <name>` passes
for real, not just `cargo check`.
