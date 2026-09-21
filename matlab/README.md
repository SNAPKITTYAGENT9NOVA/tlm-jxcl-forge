# MATLAB Certification Layer

Formal verification of state transition systems via executable certification contracts.

## Overview

The certification layer bridges **numerical computation (MATLAB)** and **formal proof (Lean 4)** by providing an executable specification of the certification predicate:

$$\operatorname{Certified}(S, x) = \operatorname{Valid}(S) \land \operatorname{Admissible}(x) \land \mathcal{I}(F(S,x))$$

A state transition is certified exactly when:
1. **Source is valid:** $\operatorname{Valid}(S) = \text{true}$
2. **Input is admissible:** $\operatorname{Admissible}(x) = \text{true}$
3. **Target satisfies invariant:** $\mathcal{I}(F(S,x)) = \text{true}$

The target state is **computed definitionally** as $F(S,x)$, not supplied by the caller. This is the critical repair from earlier unlinked forms.

## Directory Structure

```
matlab/
├── +formal/                           # Formal verification package
│   ├── certifyTransition.m            # Single-step certification
│   ├── certifyTrace.m                 # Multi-step trace certification
│   └── runCertifiedTrace.m            # Guarded execution (halt on failure)
├── tests/
│   └── testCertification.m            # Comprehensive test suite
└── README.md                          # This file
```

## Core Functions

### `formal.certifyTransition` — Single-step Certification

```matlab
certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x)
```

**Input:**
- `F` — Function handle: (S, x) → S' (state transition)
- `valid` — Function handle: S → logical (source validity predicate)
- `admissible` — Function handle: x → logical (input admissibility predicate)
- `invariant` — Function handle: S → logical (target invariant predicate)
- `S` — Current state
- `x` — Input

**Output:**
- `certificate` — Struct with fields:
  - `source` — Input state S
  - `input` — Input x
  - `target` — Computed state S' = F(S,x)
  - `sourceValid` — logical: Valid(S)
  - `inputAdmissible` — logical: Admissible(x)
  - `targetInvariant` — logical: I(F(S,x))
  - `certified` — logical: sourceValid AND inputAdmissible AND targetInvariant

**Example:**

```matlab
F = @(S, x) S + x;
valid = @(S) S >= 0 && floor(S) == S;
admissible = @(x) x >= 0 && x <= 2;
invariant = @(S) S >= 0;

cert = formal.certifyTransition(F, valid, admissible, invariant, 3, 2);
if cert.certified
    disp(['Transition certified: ', num2str(cert.source), ' + ', num2str(cert.input), ...
          ' = ', num2str(cert.target)]);
end
```

### `formal.certifyTrace` — Multi-step Trace Certification

```matlab
traceCert = formal.certifyTrace(F, valid, admissible, invariant, S0, inputs)
```

Applies `certifyTransition` to each step of an execution trace. The complete trace is certified only if **all** steps are certified.

**Output:**
- `traceCertificate` — Struct with fields:
  - `initialState` — Initial state S₀
  - `inputs` — Cell array of inputs
  - `states` — Cell array {S₀, S₁, S₂, ..., Sₙ} of all states
  - `steps` — Cell array of certification structs (one per input)
  - `certified` — logical: true if all steps certified

**Key feature:** The function generates the entire trace **even if a step fails** certification. This allows post-hoc analysis of traces that violate the contract. Use `runCertifiedTrace` if you need to halt on first failure.

**Example:**

```matlab
trace = formal.certifyTrace(F, valid, admissible, invariant, 0, {1, 2, 1});
if trace.certified
    disp('Entire trace is certified.');
    disp(trace.states);  % {0; 1; 3; 4}
else
    % Find which step failed:
    for k = 1:numel(trace.steps)
        if ~trace.steps{k}.certified
            fprintf('Step %d failed: input %d not admissible.\n', ...
                k, trace.inputs{k});
        end
    end
end
```

### `formal.runCertifiedTrace` — Guarded Execution

```matlab
states = formal.runCertifiedTrace(F, valid, admissible, invariant, S0, inputs)
```

Executes the state sequence, halting immediately on the first uncertified transition. This enforces the operational reading:

$$\text{a rejected transition cannot enter the state sequence.}$$

**Output:**
- `states` — Cell array {S₀, S₁, ..., Sₖ} of states up to (and including) the last certified transition. If the first step fails, returns {S₀}.

**Throws:** `error("formal:RejectedTransition", "Transition k was not certified.")` if any step fails.

**Example:**

```matlab
try
    states = formal.runCertifiedTrace(F, valid, admissible, invariant, 0, {1, 3, 1});
    disp('Execution completed successfully.');
catch ME
    disp(['Execution halted: ', ME.message]);
end
```

## Test Suite

Run the comprehensive test suite:

```matlab
cd matlab
results = runtests('tests');
disp(table(results))
assert(all([results.Passed]), 'Certification test suite failed.');
```

### Test Coverage

| Test | Purpose | Expected Outcome |
|------|---------|------------------|
| `testCertifiesValidAdmissibleInvariantStep` | All premises hold | Certified |
| `testTargetIsDefinitionallyComputedByStepFunction` | Target ≡ F(S,x) | Certified with correct target |
| `testRejectsInvalidSource` | Valid(S) fails | Rejected |
| `testRejectsInadmissibleInput` | Admissible(x) fails | Rejected |
| `testRejectsWhenTargetViolatesInvariant` | I(F(S,x)) fails | Rejected |
| `testNoCertificationWhenAllThreeConditionsDoNotHold` | All premises fail | Rejected |
| `testCertificationDoesNotMutateNumericState` | Purity check | Input unchanged |
| `testPredicateMustReturnLogicalScalar` | Type contract | Error on vector result |
| `testPredicateMustNotReturnNumericOne` | Contract: logical ≠ numeric | Error on numeric 1 |
| `testCertifiesEntireTrace` | Multi-step success | All states and steps certified |
| `testRejectsTraceWithOneInadmissibleInput` | Trace fails at step k | Step k rejected, trace uncertified |
| `testRejectsTraceWithInvariantViolation` | Invariant fails at step k | Step k rejected, trace uncertified |
| `testGuardedExecutorAcceptsCertifiedTrace` | Guarded execution succeeds | All states returned |
| `testGuardedExecutorRejectsUncertifiedTransition` | Guarded execution fails | Error thrown at first failure |
| `testGuardedExecutorHaltsOnFirstFailure` | Halt, don't continue | Error at step k |
| `testUnlinkedRuleWouldCertifyWrongTarget` | Regression: old defect | Demonstrates why target must be F(S,x) |
| `testBoundedStepPreservationWithIncrement` | Finite-domain exhaustive check | S ∈ [0,20], x ∈ {0,1,2}: invariant preserved |
| `testBoundedTraceCompletionWithIncrement` | All bounded traces certify | 5-step sequences on finite domain |
| `testGuardedExecutorCompletesForBoundedInputs` | Guarded execution on bounded domain | Success for all valid sequences |
| `testBoundedRejectionDetection` | Rejection on finite domain | Invalid sources and inadmissible inputs caught |

### Test Statistics

- **Core certification tests:** 9 (single-step, contract validation)
- **Trace and guarded execution tests:** 6 (trace certification, guarded execution)
- **Regression test:** 1 (old unlinked rule defect)
- **Bounded exhaustive tests:** 4 (finite-domain step preservation and rejection)
- **Total:** 20 test functions

## Predicate Contract

All predicate functions must:
1. **Return a logical scalar** — not a vector, matrix, or numeric value
2. **Be pure** — not modify state or have side effects
3. **Terminate** — always halt with a result

**Wrong:**
```matlab
valid = @(S) S >= 0;           % Returns numeric 1/0, not logical
admissible = @(x) [x >= 0, x <= 2];  % Returns vector
```

**Correct:**
```matlab
valid = @(S) S >= 0 && floor(S) == S;  % Returns logical true/false
admissible = @(x) x >= 0 && x <= 2;    % Returns single logical value
```

The certification layer enforces this via `mustReturnLogicalScalar`, which throws `error("formal:InvalidPredicateResult", ...)` if a predicate violates the contract.

## Integration with Lean 4

The executable certification in MATLAB corresponds to Lean 4 theorems:

**Lean:** Universal quantification (formal proof)
```lean
theorem certifyTransition_sound (S x S' : Type) :
  Certified(S, x, S') → S' = F(S,x) ∧ I(S')
```

**MATLAB:** Finite instances (executable verification)
```matlab
certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);
assert(isequal(certificate.target, F(S, x)));
assert(certificate.targetInvariant == true);
```

The MATLAB tests verify that the implementation conforms to the intended contract. The Lean theorems prove universal properties. Neither replaces the other.

## Extension Points

### Adding New Algorithms

1. **Define predicates** for your domain:
   ```matlab
   validState = @(S) ... logical expression ...;
   admissibleInput = @(x) ... logical expression ...;
   invariant = @(S) ... logical expression ...;
   ```

2. **Define transition function** F:
   ```matlab
   F = @(S, x) ... state update ...;
   ```

3. **Add test cases** to `testCertification.m`:
   ```matlab
   function testMyAlgorithmCertification(testCase)
       certificate = formal.certifyTransition(...);
       verifyTrue(testCase, certificate.certified);
   end
   ```

4. **Run traces** through the guarded executor:
   ```matlab
   states = formal.runCertifiedTrace(F, valid, admissible, invariant, S0, inputs);
   ```

### Bounded Exhaustive Testing

The test suite includes four bounded exhaustive tests that enumerate finite state and input domains to build confidence in step preservation and rejection detection:

1. **`testBoundedStepPreservationWithIncrement`** — Verifies S ∈ [0,20] and x ∈ {0,1,2} preserve the nonnegative invariant. All 21 × 3 = 63 combinations pass.

2. **`testBoundedTraceCompletionWithIncrement`** — Verifies that 5-step admissible input sequences from valid initial states (S₀ ∈ {0, 5, 10, 15, 20}) produce fully certified traces with all intermediate states satisfying the invariant.

3. **`testGuardedExecutorCompletesForBoundedInputs`** — Verifies that guarded execution succeeds for bounded admissible sequences, producing the correct number of states and maintaining the invariant throughout.

4. **`testBoundedRejectionDetection`** — Verifies that invalid sources (S ∉ ℤ≥0) and inadmissible inputs (x ∉ {0,1,2}) are correctly rejected in the finite domain.

**Important distinction:**

$$\text{bounded MATLAB enumeration} \neq \forall S\,\forall x,\; \mathcal{I}(S) \to \mathcal{I}(F(S,x)).$$

Bounded exhaustive tests in MATLAB provide high-confidence finite-instance regression coverage. The universal version—that step preservation holds for all states—belongs in the Lean 4 development. MATLAB establishes executable conformance; Lean proves universal correctness.

## Design Philosophy

1. **Separation of concerns:**
   - MATLAB tests the executable contract
   - Lean 4 proves universal theorems
   - Neither replaces the other

2. **Target is computed, not supplied:**
   - The old unlinked form allowed arbitrary targets satisfying the invariant
   - The corrected form computes S' = F(S,x) definitionally
   - This is the critical repair

3. **Predicates are pure functions:**
   - No implicit numeric-to-boolean coercion
   - All predicates must return logical scalars
   - Contract violations are caught at certification time

4. **Traces allow post-hoc analysis:**
   - `certifyTrace` generates complete traces even with failures
   - `runCertifiedTrace` halts on first failure
   - Use the tool appropriate to your analysis goal

## References

- [MATLAB Unit Testing](https://www.mathworks.com/help/matlab/matlab_prog/class-based-unit-tests.html)
- [Function Argument Validation](https://www.mathworks.com/help/matlab/matlab_prog/function-argument-validation-1.html)
- [Verifications and Assertions](https://www.mathworks.com/help/matlab/matlab_prog/types-of-qualifications.html)

## License

Same as parent repository (tlm-jxcl-forge).

---

**Co-Authored-By:** Claude Haiku 4.5 <noreply@anthropic.com>
