module trace

/*
 * Invariants of matlab/+formal/certifyTransition.m and certifyTrace.m.
 *   step certified  <=> valid(S) and admissible(x) and invariant(F(S,x))
 *   trace certified <=> every step certified
 */

open util/ordering[Step]

sig Value { f: Input -> lone Value }
sig Input {}
sig Valid, Inv in Value {}
sig Admissible in Input {}

sig Step { src: one Value, input: one Input, dst: one Value }

fact Transition { all s: Step | s.dst = s.input.(s.src.f) }
fact Chained { all s: Step - last | s.next.src = s.dst }

pred StepCertified[s: Step] {
  s.src in Valid and s.input in Admissible and s.dst in Inv
}
pred TraceCertified { all s: Step | StepCertified[s] }

-- T1. A certified trace visits only invariant-satisfying targets.
assert CertifiedTraceKeepsInvariant { TraceCertified implies Step.dst in Inv }
check CertifiedTraceKeepsInvariant for 6 expect 0

-- T2. Trace certification fails iff some step fails.
assert TraceIffAllSteps {
  TraceCertified iff (no s: Step | not StepCertified[s])
}
check TraceIffAllSteps for 6 expect 0

-- T3. Inductive invariant: if Inv is closed under F on admissible inputs,
--     Inv implies Valid, the trace starts in Inv and uses only admissible
--     inputs, then the whole trace is certified.
assert InductiveInvariant {
  ((all v: Inv, x: Admissible | x.(v.f) in Inv)
    and Inv in Valid
    and some Step and first.src in Inv
    and Step.input in Admissible)
  implies TraceCertified
}
check InductiveInvariant for 6 expect 0

-- C1. Checking only the final state is insufficient: a trace can end in
--     Inv while an intermediate step leaves it.
run FinalStateCheckInsufficient {
  last.dst in Inv and not TraceCertified and #Step > 1
} for 4 expect 1

-- C2. Without Inv in Valid, closure of Inv under F does not certify.
assert InductionWithoutValidity {
  ((all v: Inv, x: Admissible | x.(v.f) in Inv)
    and some Step and first.src in Inv and Step.input in Admissible)
  implies TraceCertified
}
check InductionWithoutValidity for 4 expect 1
