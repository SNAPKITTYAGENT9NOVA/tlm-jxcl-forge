module dula

/*
 * Invariants of tools/emacs/dula-lean-alloy.el:
 *   - dula-recursive-assert: depth increases by one per parent hop and
 *     is bounded by dula-recursion-limit (32; modelled as a small bound).
 *   - dula-counterlemma-create: status is exactly one of
 *     counterexample-found / no-counterexample-found, decided by the
 *     Alloy outcome of the counterformula check.
 */

abstract sig Outcome {}
one sig Sat, Unsat, Failed extends Outcome {}

abstract sig Status {}
one sig Pending, CounterexampleFound, NoCounterexampleFound extends Status {}

one sig Limit { value: one Int }

sig Assertion {
  parent: lone Assertion,
  depth: one Int,
  outcome: lone Outcome,
  status: one Status
}

fact Limit4 { Limit.value = 4 }

fact Recursion {
  no a: Assertion | a in a.^parent
  all a: Assertion | no a.parent implies a.depth = 0
  all a: Assertion | some a.parent implies a.depth = plus[a.parent.depth, 1]
}

-- The corrected classifier: only SAT means a counterexample; UNSAT means
-- none within scope; a failed run stays Pending (never "verified").
fact Classifier {
  all a: Assertion {
    a.outcome = Sat implies a.status = CounterexampleFound
    a.outcome = Unsat implies a.status = NoCounterexampleFound
    (no a.outcome or a.outcome = Failed) implies a.status = Pending
  }
}

-- R1. Depth never goes negative.
assert DepthNonNegative { all a: Assertion | a.depth >= 0 }
check DepthNonNegative for 5 but 5 int expect 0

-- R2. Status is a function of the outcome (no crossed classification).
assert NoCrossedClassification {
  no a: Assertion | a.outcome = Unsat and a.status = CounterexampleFound
  no a: Assertion | a.outcome = Sat and a.status = NoCounterexampleFound
}
check NoCrossedClassification for 5 but 5 int expect 0

-- R3. A failed Alloy run is never reported as evidence either way.
assert FailureIsNotEvidence {
  all a: Assertion | a.outcome = Failed implies a.status = Pending
}
check FailureIsNotEvidence for 5 but 5 int expect 0

-- C1. Without the limit check, parent chains exceed dula-recursion-limit;
--     this is why dula-recursive-assert must raise at depth > limit.
run ChainExceedsLimit {
  some a: Assertion | a.depth > Limit.value
} for 7 but 5 int expect 1
