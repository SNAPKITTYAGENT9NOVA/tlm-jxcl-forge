# Hardened invariants and counter-algorithms

This directory collects every invariant and counterexample search found
in the repository's formal-verification layer (`alloy/`, `matlab/`,
`lean/`, `tools/emacs/`). Each one is restated as an Alloy command with an
explicit `expect`, and mirrored as an executable Crystal spec.

- **Invariant** (`expect 0`): Alloy must find **no** counterexample
  within the stated scope.
- **Counter-algorithm** (`expect 1`): Alloy **must** find an instance.
  This proves that a naive or superseded claim is false, and that the
  search can actually reach counterexamples, so a green run is not
  vacuous.

Alloy results are bounded evidence (no counterexample up to the scope).
Lean remains the proof authority.

## Layout

| Path | Contents |
|---|---|
| `core.als` | Hardened semantic core: well-founded propositions, DAG dependencies, free `Atom`s |
| `lemmas.als` | Lemma invariants I1–I11, counter-algorithms C1–C4, non-vacuity run V1 |
| `decomposition.als` | LU/QR/SVD/Cholesky certification rule D1–D4, C1–C3 |
| `trace.als` | Transition/trace certification T1–T3, C1–C2 |
| `dula.als` | DULA classifier and recursion invariants R1–R3, C1 |
| `check.sh` | Runs every Alloy command and fails on any result that differs from its `expect` |
| `crystal/` | Crystal shard mirroring all of the above (`crystal spec`) |

## Running

```sh
# Alloy 6 (tested with 6.2.0): 32 commands
ALLOY_JAR=/path/to/org.alloytools.alloy.dist.jar ./check.sh

# Crystal (tested with 1.14.0; needs libevent-dev to link): 29 examples
cd crystal && crystal spec
```

## Inventory

| Source | Invariant / counter-algorithm | Alloy | Crystal spec |
|---|---|---|---|
| `FreehandLemmas.als` `NoCyclicLemmaDependencies` | Dependencies form a DAG | `lemmas` I1 (+ `core` fact) | `logic_spec` I1 |
| `FreehandLemmas.als` `ViolatesLemma` / `LemmaIsSound` | Counterexample ⇔ unsound | `lemmas` I2 | `logic_spec` I2 |
| `FreehandLemmas.als` `ExcludedMiddleIsSound` | P ∨ ¬P; ¬(P ∧ ¬P) | `lemmas` I3, I4 | `logic_spec` I3/I4 |
| `FreehandLemmas.als` `ExFalsoIsSound` | Contradictory assumptions ⇒ sound | `lemmas` I5 | `logic_spec` I5 |
| `FreehandLemmas.als` `ExampleTransitivity`, DSL `Transitivity` | Modus ponens, hypothetical syllogism | `lemmas` I6, I7 | `logic_spec` I6/I7 |
| DSL `De_Morgan_And`, `Contrapositive` | De Morgan, contrapositive | `lemmas` I8, I9 | `logic_spec` I8/I9 |
| `FreehandLemmas-Semantics.md` monotonicity | Adding assumptions preserves soundness | `lemmas` I10 | `logic_spec` I10 |
| `FreehandLemmas.als` `InDomain`, `ElementsInSameDomain` | Structural atoms are state-invariant | `lemmas` I11 | — |
| original `ExFalsoIsSound` | "Every lemma is sound" is **false** | `lemmas` C1 | `logic_spec` C1 |
| — | Affirming the consequent; converse ≠ implication | `lemmas` C2, C3 | `logic_spec` C2/C3 |
| `ViolatesLemma` | A non-vacuous counterexample exists | `lemmas` C4 | `logic_spec` C1 |
| `matlab/+*/certifyDecomposition.m` | Certified ⇔ all required invariants within tolerance | `decomposition` D1, C2 | `certify_spec` D1, C2 |
| `matlab/+*/certifyDecomposition.m` | Reconstruction always required; 3–5 invariants per kind | `decomposition` D2, D3 | `certify_spec` D2/D3 |
| `matlab/+cholesky`, `QR.lean` `IsUpperTriangular` | Cholesky needs SPD; reconstruction alone ≠ certified | `decomposition` D4, C1 | `certify_spec` D4, C1 |
| `matlab/+formal/certifyTransition.m`, `certifyTrace.m` | Step/trace certification | `trace` T1, T2 | `trace_spec` T1/T2 |
| `certifyTrace.m` (induction) | Closed invariant ∧ Inv ⊆ Valid ⇒ every trace certified | `trace` T3, C2 | `trace_spec` T3, C2 |
| `certifyTrace.m` | Checking only the final state is insufficient | `trace` C1 | `trace_spec` C1 |
| `dula-lean-alloy.el` `dula-counterlemma-create` | SAT ⇒ counterexample, UNSAT ⇒ none, failure ⇒ pending | `dula` R2, R3 | `dula_spec` R2/R3 |
| `dula-lean-alloy.el` `dula-recursive-assert` | Depth starts at 0, +1 per parent, bounded by the limit | `dula` R1, C1 | `dula_spec` R1, C1 |

The Lean theorems in `lean/MathlibMatrixFormalization/` (for example
`qr_reconstruction`, `orthogonal_mul_orthogonal`, `solve_correct`) are
the proof-level counterparts of the decomposition invariants. They are
exercised numerically by the Crystal LU/QR/Cholesky kernels and
`certify_spec`, not restated as Alloy.

## Defects found and fixed

1. **`dula-lean-alloy.el` inverted the classifier.** It matched the
   substrings `"SAT"` and `"found"` before checking for `UNSAT`, so every
   verified result (`UNSAT`, "No counterexample found") was recorded as
   `counterexample-found`. A failed run was also recorded as
   `no-counterexample-found`. Now only a whole-word `SAT` or "counterexample/instance found" counts as a counterexample, and a failed run stays
   `pending`. Pinned by `dula.als` R2/R3 and `dula_spec`; the old
   behaviour is kept as `Dula.legacy_classify` to document the counterexample.
2. **`FreehandLemmas.als` did not parse.** A trailing `end` line was a
   syntax error, so none of its commands could run. Removed.
3. **`FreehandLemmas.als` claims that fail.** `NoCyclicLemmaDependencies`
   and all four `ExFalsoIsSound` checks return counterexamples (no DAG
   fact, and the assertion claims every lemma is sound). They are
   annotated in place. The corrected forms are `core.als` `DependencyDAG` and
   `lemmas.als` I1/I5.
4. **Ill-founded propositions.** Without a well-foundedness fact, a
   `Not` can be its own operand, and `holds = State - holds` then forces
   the state space to be empty. `core.als` adds `WellFounded`.

## Defects found, not fixed here (MATLAB not available to test)

- `matlab/+svd/certifyDecomposition.m` compares the full `U'*U` (m×m)
  against `eye(min(m,n))`, and builds `diag(diag(S))` as a square matrix.
  Both throw on non-square input. The Crystal `Certify.svd` sizes the
  identity from `U`'s columns and handles rectangular `S`
  (`certify_spec` "including rectangular").
- All four MATLAB certifiers divide by `norm(A,'fro')`, so a zero matrix
  gives `0/0 = NaN` and silently fails certification. The Crystal port
  divides by `max(norm, eps)` instead.
