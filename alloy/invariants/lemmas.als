module lemmas

/*
 * Lemma-level invariants (expect 0: no counterexample may exist) and
 * counter-algorithms (expect 1: a counterexample MUST be found, proving
 * the naive claim is false and the search machinery is live).
 */

open core

-- I1. Dependency graph is acyclic.
assert DependenciesAcyclic { no l: Lemma | l in l.^dependencies }
check DependenciesAcyclic for 6 expect 0

-- I2. A counterexample exists exactly when the lemma is unsound.
assert CounterexampleIffUnsound {
  all l: Lemma | LemmaIsSound[l] iff (no s: State | ViolatesLemma[l, s])
}
check CounterexampleIffUnsound for 5 expect 0

-- I3. Excluded middle: P or not P holds everywhere.
assert ExcludedMiddle {
  all o: Or | (o.right in Not and o.right.operand = o.left) implies o.holds = State
}
check ExcludedMiddle for 5 expect 0

-- I4. Non-contradiction: P and not P holds nowhere.
assert NonContradiction {
  all a: And | (a.right in Not and a.right.operand = a.left) implies no a.holds
}
check NonContradiction for 5 expect 0

-- I5. Ex falso, correctly stated: contradictory assumptions make a lemma sound.
--     (The original ExFalsoIsSound asserted every lemma is sound.)
assert ExFalso { all l: Lemma | Contradictory[l] implies LemmaIsSound[l] }
check ExFalso for 5 expect 0

-- I6. Modus ponens.
assert ModusPonens {
  all l: Lemma, i: Implies |
    (i + i.antecedent in l.assumptions and l.conclusion = i.consequent)
      implies LemmaIsSound[l]
}
check ModusPonens for 5 expect 0

-- I7. Hypothetical syllogism (transitivity of implication).
assert HypotheticalSyllogism {
  all l: Lemma, i1, i2, c: Implies |
    (i1 + i2 in l.assumptions and i1.consequent = i2.antecedent
      and l.conclusion = c and c.antecedent = i1.antecedent
      and c.consequent = i2.consequent)
      implies LemmaIsSound[l]
}
check HypotheticalSyllogism for 6 expect 0

-- I8. De Morgan: not (P and Q) = not P or not Q.
assert DeMorgan {
  all n: Not, o: Or |
    (n.operand in And and o.left in Not and o.right in Not
      and o.left.operand = n.operand.(And <: left) and o.right.operand = n.operand.(And <: right))
      implies n.holds = o.holds
}
check DeMorgan for 6 expect 0

-- I9. Contrapositive: (P => Q) = (not Q => not P).
assert Contrapositive {
  all i, j: Implies |
    (j.antecedent in Not and j.consequent in Not
      and j.antecedent.operand = i.consequent and j.consequent.operand = i.antecedent)
      implies i.holds = j.holds
}
check Contrapositive for 6 expect 0

-- I10. Monotonicity: adding assumptions never breaks a sound lemma.
assert AssumptionWeakening {
  all l1, l2: Lemma |
    (l1.conclusion = l2.conclusion and l1.assumptions in l2.assumptions
      and LemmaIsSound[l1]) implies LemmaIsSound[l2]
}
check AssumptionWeakening for 5 expect 0

-- I11. Structural atoms are state-invariant (all or nothing).
assert StructuralAtomsConstant {
  all p: InDomain + ElementsInSameDomain | p.holds = State or no p.holds
}
check StructuralAtomsConstant for 5 expect 0

-- C1. The original ExFalsoIsSound claim ("every lemma is sound") is false.
assert NaiveEveryLemmaSound { all l: Lemma | LemmaIsSound[l] }
check NaiveEveryLemmaSound for 4 expect 1

-- C2. Affirming the consequent is unsound: (P => Q), Q does not give P.
assert AffirmingConsequent {
  all l: Lemma, i: Implies |
    (i + i.consequent in l.assumptions and l.conclusion = i.antecedent)
      implies LemmaIsSound[l]
}
check AffirmingConsequent for 5 expect 1

-- C3. The converse of an implication is not equivalent to it.
assert ConverseEquivalent {
  all i, j: Implies |
    (j.antecedent = i.consequent and j.consequent = i.antecedent) implies i.holds = j.holds
}
check ConverseEquivalent for 5 expect 1

-- C4. A genuine (non-vacuous) counterexample witness can be produced.
run GenuineCounterexample {
  some l: Lemma, s: State | ViolatesLemma[l, s] and not Contradictory[l]
} for 4 expect 1

-- V1. Non-vacuity: every invariant's premise is satisfiable together with
--     a non-empty state space, so no UNSAT above is an artefact of the facts.
run NonVacuous {
  some State
  some o: Or | o.right in Not and o.right.operand = o.left
  some n: Not, o: Or | n.operand in And and o.left in Not and o.right in Not
    and o.left.operand = n.operand.(And <: left)
    and o.right.operand = n.operand.(And <: right)
  some l: Lemma, i: Implies | i + i.antecedent in l.assumptions and l.conclusion = i.consequent
  some l: Lemma | Contradictory[l] and some l.assumptions
  some l: Lemma | some l.dependencies
} for 8 expect 1
