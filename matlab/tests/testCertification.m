function tests = testCertification
%testCertification Comprehensive test suite for the certification layer.
%
% Tests the executable certification contract:
%   Certified(S, x) ⟺ Valid(S) ∧ Admissible(x) ∧ I(F(S,x))
%
% The target state S' is computed definitionally as F(S,x), not caller-supplied.
%
% Test organization:
%   1. Successful certification (all premises hold)
%   2. Target computation (S' = F(S,x) definitionally)
%   3. Rejection due to invalid source
%   4. Rejection due to inadmissible input
%   5. Rejection due to invariant violation
%   6. Rejection when all premises fail
%   7. Purity checks (no mutation of input state)
%   8. Predicate contract validation
%   9. Trace certification
%  10. Guarded execution
%  11. Regression test: unlinked rule defect

tests = functiontests(localfunctions);
end

%% ============================================================================
% Single-Step Certification Tests
%% ============================================================================

function testCertifiesValidAdmissibleInvariantStep(testCase)
% When source is valid, input is admissible, and target satisfies invariant,
% the transition is certified.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S = 3;
x = 2;

certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);

verifyTrue(testCase, certificate.sourceValid);
verifyTrue(testCase, certificate.inputAdmissible);
verifyTrue(testCase, certificate.targetInvariant);
verifyTrue(testCase, certificate.certified);

verifyEqual(testCase, certificate.source, 3);
verifyEqual(testCase, certificate.input, 2);
verifyEqual(testCase, certificate.target, 5);
end

function testTargetIsDefinitionallyComputedByStepFunction(testCase)
% The target state must be computed as F(S,x), not supplied independently.
% This is the critical repair from the unlinked form.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S = 7;
x = 1;

certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);

% Target must equal F(S,x)
verifyEqual(testCase, certificate.target, F(S, x));
verifyEqual(testCase, certificate.target, 8);
verifyTrue(testCase, certificate.certified);
end

%% ============================================================================
% Rejection Tests: Individual Premise Failure
%% ============================================================================

function testRejectsInvalidSource(testCase)
% When Valid(S) fails, certification fails (even if other premises hold).

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S = -1;  % Invalid: negative state
x = 1;

certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);

verifyFalse(testCase, certificate.sourceValid);
verifyTrue(testCase, certificate.inputAdmissible);
verifyTrue(testCase, certificate.targetInvariant);
verifyFalse(testCase, certificate.certified);
end

function testRejectsInadmissibleInput(testCase)
% When Admissible(x) fails, certification fails.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S = 4;
x = 3;  % Inadmissible: exceeds bound of 2

certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);

verifyTrue(testCase, certificate.sourceValid);
verifyFalse(testCase, certificate.inputAdmissible);
verifyTrue(testCase, certificate.targetInvariant);
verifyFalse(testCase, certificate.certified);
end

function testRejectsWhenTargetViolatesInvariant(testCase)
% When I(F(S,x)) fails, certification fails.

F = @decrementStep;
valid = @validNaturalState;
admissible = @unitInput;
invariant = @nonnegativeState;

S = 0;
x = 1;  % F(0,1) = 0 - 1 = -1, which violates I

certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);

verifyTrue(testCase, certificate.sourceValid);
verifyTrue(testCase, certificate.inputAdmissible);
verifyFalse(testCase, certificate.targetInvariant);
verifyFalse(testCase, certificate.certified);

verifyEqual(testCase, certificate.target, -1);
end

function testNoCertificationWhenAllThreeConditionsDoNotHold(testCase)
% When every premise fails, certification fails.

F = @decrementStep;
valid = @validNaturalState;
admissible = @unitInput;
invariant = @nonnegativeState;

S = -2;  % Invalid source
x = 4;   % Inadmissible input (not 1)

certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);

verifyFalse(testCase, certificate.sourceValid);
verifyFalse(testCase, certificate.inputAdmissible);
verifyFalse(testCase, certificate.targetInvariant);
verifyFalse(testCase, certificate.certified);
end

%% ============================================================================
% Purity and Contract Tests
%% ============================================================================

function testCertificationDoesNotMutateNumericState(testCase)
% Certification is a pure function: it must not modify its inputs.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S = 5;
x = 2;

formal.certifyTransition(F, valid, admissible, invariant, S, x);

% Input values must remain unchanged
verifyEqual(testCase, S, 5);
verifyEqual(testCase, x, 2);
end

function testPredicateMustReturnLogicalScalar(testCase)
% Predicates must return a logical scalar, not a vector or matrix.

F = @incrementStep;
valid = @(S) S >= 0;
admissible = @(x) [x >= 0, x <= 2];  % Returns [true, true] vector
invariant = @(S) S >= 0;

verifyError(testCase, ...
    @() formal.certifyTransition(F, valid, admissible, invariant, 0, 1), ...
    "formal:InvalidPredicateResult");
end

function testPredicateMustNotReturnNumericOne(testCase)
% Predicates must return actual logical values, not numeric 1.
% This prevents implicit numeric-as-Boolean coercion.

F = @incrementStep;
valid = @(S) 1;  % Returns numeric 1, not logical true
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

verifyError(testCase, ...
    @() formal.certifyTransition(F, valid, admissible, invariant, 0, 1), ...
    "formal:InvalidPredicateResult");
end

%% ============================================================================
% Trace Certification Tests
%% ============================================================================

function testCertifiesEntireTrace(testCase)
% When all steps are certified, the entire trace is certified.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S0 = 0;
inputs = {1, 2, 0, 1};

traceCertificate = formal.certifyTrace( ...
    F, valid, admissible, invariant, S0, inputs);

verifyTrue(testCase, traceCertificate.certified);

% Expected state sequence: 0 -> 1 -> 3 -> 3 -> 4
expectedStates = {0; 1; 3; 3; 4};
verifyEqual(testCase, traceCertificate.states, expectedStates);

% Every step must be certified
for r = 1:numel(traceCertificate.steps)
    verifyTrue(testCase, traceCertificate.steps{r}.certified);
    verifyTrue(testCase, traceCertificate.steps{r}.targetInvariant);
end
end

function testRejectsTraceWithOneInadmissibleInput(testCase)
% If any step fails certification, the entire trace is uncertified.
% The trace continues past the failure (for post-hoc analysis).

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S0 = 0;
inputs = {1, 3, 1};  % Input 3 is inadmissible (max is 2)

traceCertificate = formal.certifyTrace( ...
    F, valid, admissible, invariant, S0, inputs);

verifyFalse(testCase, traceCertificate.certified);

% Step 1 is certified: 0 + 1 = 1 ✓
verifyTrue(testCase, traceCertificate.steps{1}.certified);
% Step 2 is not: 1 + 3 with 3 inadmissible ✗
verifyFalse(testCase, traceCertificate.steps{2}.certified);
verifyFalse(testCase, traceCertificate.steps{2}.inputAdmissible);
% Step 3 continues anyway: 4 + 1 = 5 (for analysis)
verifyTrue(testCase, traceCertificate.steps{3}.certified);

% But overall trace is uncertified due to step 2
verifyFalse(testCase, traceCertificate.certified);
end

function testRejectsTraceWithInvariantViolation(testCase)
% If any step violates the invariant, the trace is uncertified.

F = @decrementStep;
valid = @validNaturalState;
admissible = @unitInput;
invariant = @nonnegativeState;

S0 = 1;
inputs = {1, 1};

traceCertificate = formal.certifyTrace( ...
    F, valid, admissible, invariant, S0, inputs);

verifyFalse(testCase, traceCertificate.certified);

% Step 1: 1 - 1 = 0 ✓ (still nonnegative)
verifyTrue(testCase, traceCertificate.steps{1}.certified);
% Step 2: 0 - 1 = -1 ✗ (violates invariant)
verifyFalse(testCase, traceCertificate.steps{2}.certified);
verifyFalse(testCase, traceCertificate.steps{2}.targetInvariant);

% States: {1; 0; -1} (computed despite failure)
verifyEqual(testCase, traceCertificate.states, {1; 0; -1});

% Overall trace is uncertified
verifyFalse(testCase, traceCertificate.certified);
end

%% ============================================================================
% Guarded Execution Tests
%% ============================================================================

function testGuardedExecutorAcceptsCertifiedTrace(testCase)
% When all steps are certified, guarded execution completes normally.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

states = formal.runCertifiedTrace( ...
    F, valid, admissible, invariant, 0, {1, 2, 0});

% Expected states: 0 -> 1 -> 3 -> 3
verifyEqual(testCase, states, {0; 1; 3; 3});
end

function testGuardedExecutorRejectsUncertifiedTransition(testCase)
% When any step fails certification, guarded execution halts and throws.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

verifyError(testCase, ...
    @() formal.runCertifiedTrace( ...
        F, valid, admissible, invariant, 0, {1, 3, 1}), ...
    "formal:RejectedTransition");
end

function testGuardedExecutorHaltsOnFirstFailure(testCase)
% Guarded execution must halt on the first uncertified transition.

F = @decrementStep;
valid = @validNaturalState;
admissible = @unitInput;
invariant = @nonnegativeState;

% States would be: 5 -> 4 (ok) -> 3 (ok) -> -2 (fail)
verifyError(testCase, ...
    @() formal.runCertifiedTrace( ...
        F, valid, admissible, invariant, 5, {1, 1, 8}), ...
    "formal:RejectedTransition");
end

%% ============================================================================
% Regression Test: Unlinked Rule Defect
%% ============================================================================

function testUnlinkedRuleWouldCertifyWrongTarget(testCase)
% REGRESSION: The old unlinked certification form would permit arbitrary
% targets as long as they satisfied the invariant, severing the link
% between the source state and the computed target.
%
% This test demonstrates the defect. It should PASS, showing that:
%   Valid(0) ∧ Admissible(1) ∧ I(100)
% would certify an arbitrary target 100, despite F(0,1) = 1 ≠ 100.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

S = 0;
x = 1;
wrongTarget = 100;

% Old, unsound rule (test-local only, never exported)
certificate = oldUnlinkedCertification( ...
    valid, admissible, invariant, S, x, wrongTarget);

% The old rule would incorrectly certify this
verifyTrue(testCase, certificate.certified);
% But the target is wrong:
verifyNotEqual(testCase, certificate.target, F(S, x));
verifyNotEqual(testCase, certificate.target, 1);

% The correct implementation rejects this by design:
% target is always F(S,x), so this mismatch cannot occur.
correctCertificate = formal.certifyTransition( ...
    F, valid, admissible, invariant, S, x);

verifyTrue(testCase, correctCertificate.certified);
verifyEqual(testCase, correctCertificate.target, 1);  % Always F(S,x)
end

%% ============================================================================
% Step Functions (Fixtures)
%% ============================================================================

function Snext = incrementStep(S, x)
%incrementStep Transition function F(S, x) = S + x
Snext = S + x;
end

function Snext = decrementStep(S, x)
%decrementStep Transition function F(S, x) = S - x
Snext = S - x;
end

%% ============================================================================
% Predicate Functions (Fixtures)
%% ============================================================================

function tf = validNaturalState(S)
%validNaturalState Valid(S) iff S is a nonnegative integer
tf = isscalar(S) && isfinite(S) && S >= 0 && floor(S) == S;
end

function tf = nonnegativeState(S)
%nonnegativeState Invariant: I(S) iff S is nonnegative
tf = isscalar(S) && isfinite(S) && S >= 0;
end

function tf = smallNonnegativeInput(x)
%smallNonnegativeInput Admissible(x) iff x ∈ {0, 1, 2}
tf = isscalar(x) && isfinite(x) && floor(x) == x && x >= 0 && x <= 2;
end

function tf = unitInput(x)
%unitInput Admissible(x) iff x = 1
tf = isequal(x, 1);
end

%% ============================================================================
% Unsound Certification (for Regression Test)
%% ============================================================================

function certificate = oldUnlinkedCertification(valid, admissible, invariant, S, x, arbitraryTarget)
%oldUnlinkedCertification UNSOUND form: certifies (S,x) -> arbitraryTarget.
%
% This is the defective form from earlier development. It permits any target
% that satisfies the invariant, severing the link to F(S,x). DO NOT USE IN
% PRODUCTION. Kept test-local only for regression testing.
%
% The defect is that Certified(S, x, T) = Valid(S) ∧ Admissible(x) ∧ I(T)
% permits T to be any invariant-satisfying state, not necessarily F(S,x).

certificate = struct( ...
    "source", S, ...
    "input", x, ...
    "target", arbitraryTarget, ...
    "certified", logical(valid(S) && admissible(x) && invariant(arbitraryTarget)));
end

%% ============================================================================
% Bounded Exhaustive Testing
%% ============================================================================

function testBoundedStepPreservationWithIncrement(testCase)
% Finite-instance verification: for S in [0, 20] and x in {0, 1, 2},
% verify that Valid(S) and Admissible(x) imply Invariant(F(S,x)).
%
% This is a high-confidence finite check, not a universal proof.
% The universal version: ∀ S ∀ x, I(S) → I(F(S,x)) belongs in Lean.

F = @incrementStep;
invariant = @nonnegativeState;

for S = 0:20
    for x = 0:2
        Snext = F(S, x);
        verifyTrue(testCase, invariant(Snext), ...
            sprintf("Invariant failed at S=%d, x=%d: F(S,x)=%d", S, x, Snext));
    end
end
end

function testBoundedTraceCompletionWithIncrement(testCase)
% Verify that all bounded sequences of admissible inputs from valid initial
% states produce certified traces over the finite domain.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

initialStates = [0, 5, 10, 15, 20];
for S0 = initialStates
    if ~valid(S0), continue; end

    inputs = {0, 1, 2, 1, 0};  % 5-step sequence of admissible inputs

    traceCert = formal.certifyTrace(F, valid, admissible, invariant, S0, inputs);

    % All steps must be certified
    verifyTrue(testCase, traceCert.certified, ...
        sprintf("Trace from S0=%d failed to certify", S0));

    % All computed states must satisfy invariant
    for k = 1:numel(traceCert.states)
        verifyTrue(testCase, invariant(traceCert.states{k}), ...
            sprintf("State at step %d violates invariant (S0=%d)", k-1, S0));
    end
end
end

function testGuardedExecutorCompletesForBoundedInputs(testCase)
% Verify that guarded execution succeeds for all bounded admissible sequences
% on valid initial states.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

testSequences = {
    {0, [0, 1, 2]},
    {5, [1, 1, 0, 2]},
    {10, [2, 0, 1]},
    {20, [0, 0, 0, 0]}
};

for k = 1:numel(testSequences)
    S0 = testSequences{k}{1};
    inputs = testSequences{k}{2};

    % Should not throw
    states = formal.runCertifiedTrace(F, valid, admissible, invariant, S0, inputs);

    % Should produce correct number of states
    verifyLength(testCase, states, numel(inputs) + 1);

    % First state must be initial state
    verifyEqual(testCase, states{1}, S0);

    % All states must satisfy invariant
    for s = states
        verifyTrue(testCase, invariant(s{1}));
    end
end
end

function testBoundedRejectionDetection(testCase)
% Verify that invalid sources and inadmissible inputs are caught
% in the bounded domain.

F = @incrementStep;
valid = @validNaturalState;
admissible = @smallNonnegativeInput;
invariant = @nonnegativeState;

% Test rejection due to invalid source
invalidSources = [-1, -5, 100.5];  % Not in Z_≥0
for S = invalidSources
    certificate = formal.certifyTransition(F, valid, admissible, invariant, S, 1);
    verifyFalse(testCase, certificate.sourceValid, ...
        sprintf("Source S=%g should be invalid", S));
    verifyFalse(testCase, certificate.certified);
end

% Test rejection due to inadmissible input
inadmissibleInputs = [3, 4, 5, -1];  % Not in {0,1,2}
S = 5;
for x = inadmissibleInputs
    certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x);
    verifyFalse(testCase, certificate.inputAdmissible, ...
        sprintf("Input x=%d should be inadmissible", x));
    verifyFalse(testCase, certificate.certified);
end
end
