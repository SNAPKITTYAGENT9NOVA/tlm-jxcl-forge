# spec/

Two TLA+ modules, added verbatim as supplied, as reference/design
material alongside the `tensor-forge` Rust crates. **Neither has been
run through SANY/TLC in this repository** (no `.cfg` file, no CI job),
so treat them as illustrative pseudocode, not a checked specification,
until someone does that pass.

- [`NArrayLogic.tla`](./NArrayLogic.tla) — a partition/product/difference/
  transform/composition pipeline over an `N`-length array, sealed by an
  `Invariant` (sortedness) check and an append-only `worm` log.
- [`SWARM.tla`](./SWARM.tla) — a three-agent (generator/verifier/
  temporal-arbiter) commit loop with a `MaxLines`-bounded `Loop` →
  `Finalize`/`HaltFail` pipeline.

## Known gaps if you do run these through SANY

- `SWARM.tla`'s `Loop`, `Finalize`, `HaltFail`, `Commit`, and
  `Immutable` reference `r` as a free identifier rather than taking it
  as a parameter (contrast `Generator(r)`/`TemporalStep(r)`, which do);
  `SwarmStep(r)` calls them with no argument, so `r` is unbound at each
  call site as written.
- `SWARM.tla` calls `Syntax`, `Type`, `ProofOf`, `F`, `Provable`,
  `MPL`, `PURE_MATH`, and `VerifiedOutput` without defining or
  `EXTENDS`-ing any of them.
- `NArrayLogic.tla`'s `Difference` subtracts into `Nat`, which is
  partial below zero; `TLC` will flag any state where `x[i] < y[i]`
  unless the module is extended to (or restricted to) `Int`.

None of this is a Rust-crate concern — `tensor-core`/`tensor-linalg`
don't depend on or generate these modules — it's noted here so the
gap between "added to the repo" and "formally verified" stays visible.
