# spec/

Two closed, `TLC`-checked TLA+ state machines, added alongside the
`tensor-forge` Rust crates as design/reference material (neither is a
build dependency of `tensor-core`/`tensor-linalg`/`tensor-forge`).

- [`NArrayLogic.tla`](./NArrayLogic.tla) — a partition/product/
  difference/transform/composition pipeline over an `N`-length array,
  sealed by an `Invariant` (sortedness) check and an append-only
  `worm` log.
- [`SWARM.tla`](./SWARM.tla) — a three-agent (generator/verifier/
  temporal-arbiter) generate → verify → temporal-commit loop over
  `MaxLines` candidate lines, ending in `FINALIZE` (every line
  verified) or `HALT` (a rejected line triggers `Rollback` and latches
  `TemporalLock`).

## Verification status

Both modules are **complete, total state machines**: every reachable
state has an enabled transition (including an absorbing self-loop once
terminal, so reaching `"sealed"`/`"failed"` or `"FINALIZE"`/`"HALT"`
is a rest state, not a `TLC` deadlock), every operator they use is
defined in the module (no free variables, no undefined operators), and
each carries a `TypeOK` invariant, an additional safety invariant, and
a `<>Terminal`-shaped liveness property under `WF_vars(RealNext)`
fairness. Model-checked with `TLC2` (`tla2tools.jar`, the `tlaplus/
tlaplus` release build) against every `.cfg` in this directory:

| Module | Config | Result |
|---|---|---|
| `NArrayLogic.tla` | `NArrayLogic.cfg` (`N = 4`) | 9 states, `TypeOK` + `EventuallyTerminal` hold, no error |
| `SWARM.tla` | `SWARM.cfg` (`MaxLines = 3`, all 3 lines good) | 14 states, reaches `FINALIZE`; `TypeOK` + `Safety` + `EventuallyTerminal` hold, no error |
| `SWARM.tla` | `SWARM_rollback.cfg` (`MaxLines = 3`, line 2 rejected) | 8 states, reaches `HALT` via `Rollback` with `temporalLock = 1 /\ a1 = "ROLLBACK"`; same three properties hold, no error |

Reproduce locally:

```sh
curl -fsSL -o tla2tools.jar \
  https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar
java -cp tla2tools.jar tlc2.TLC -workers auto -config NArrayLogic.cfg NArrayLogic.tla
java -cp tla2tools.jar tlc2.TLC -workers auto -config SWARM.cfg SWARM.tla
java -cp tla2tools.jar tlc2.TLC -workers auto -config SWARM_rollback.cfg SWARM.tla
```

## What changed from the first draft

The modules as originally supplied were **not** closed, checkable
specs — `SWARM.tla` in particular referenced `r` as a free variable in
several operators (`Loop`, `Finalize`, `HaltFail`, `Commit`,
`Immutable`) rather than a parameter, called several operators/
constants (`Syntax`, `Type`, `ProofOf`, `F`, `Provable`, `MPL`,
`PURE_MATH`, `VerifiedOutput`) that were never defined, and modeled
`A1`/`L`/`State`/`Time`/`Hash`/`Proof`/`Verified` as functions from all
of `Nat`, an infinite domain `TLC` cannot enumerate. This revision:

- Replaces the `Nat`-indexed infinite functions with ordinary
  variables holding the *current* round's value (`line`, `time`,
  `hashTrail` as a growing but always-finite `Seq`, `verified` as a
  function over the fixed finite `1..MaxLines`), so the reachable
  state space is finite.
- Abstracts the real verifier (`Syntax /\ Type /\ Invariant /\
  ProofOf`) as `line \in GoodLines`, a model constant — a standard TLA+
  move for "some external check holds," and lets the two `.cfg`s above
  drive the success and rollback paths deliberately rather than by
  chance.
- Fixes `NArrayLogic.tla`'s `Difference` to truncate at zero (`IF x[i]
  >= y[i] THEN x[i] - y[i] ELSE 0`) rather than subtracting into `Nat`,
  matching the `⊖` (monus) operator's definition in the original boxed
  spec this module was drawn from.
- Fixes a real parameter/variable name collision (`Product(p)`/
  `Closure(p)` shadowing the `p` `VARIABLE`) and a `TLC` runtime
  limitation (comparing an integer against a string within the same
  set literal throws rather than returning `FALSE` in this `TLC`
  version) that `SWARM.tla`'s `a1 \in {0, 1, "ROLLBACK"}` tripped;
  `a1` now ranges over `{"IDLE", "RAISED", "ROLLBACK"}` instead of
  mixing integers and a string.
- Adds the missing `WF_vars(RealNext)`/`Idle`-self-loop split both
  modules needed: a fairness obligation on an action whose own effect
  is `UNCHANGED vars` can never be satisfied once it's the only action
  left enabled, so the absorbing terminal self-loop is deliberately
  excluded from what `WF_vars` ranges over.
