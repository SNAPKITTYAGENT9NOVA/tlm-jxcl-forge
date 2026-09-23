/**
 * FREEHAND LEMMAS IN ALLOY — State-Based Semantic Framework
 *
 * A rigorous formalization of human-authored lemmas with explicit semantics
 * where each proposition denotes a set of model states.
 *
 * Key distinction from informal lemmas:
 *   Informal: "If x is positive, then x > 0"
 *   Formal:   ⟦x is positive⟧ ⊆ ⟦x > 0⟧
 *   where ⟦·⟧ : Proposition → ℘(State)
 */

module FreehandLemmas

/* ============================================================================
   TIER 1: SEMANTIC DOMAIN
   ============================================================================ */

/**
 * A Domain represents a universe of discourse.
 * Elements belong to exactly one domain.
 */
sig Domain {}

/**
 * An Element is an atomic entity within a Domain.
 */
sig Element {
    belongsTo: one Domain
}

/* ============================================================================
   TIER 2: STATE SPACE
   ============================================================================ */

/**
 * A State is a possible configuration of the model.
 *
 * In Alloy's semantics, a State is implicitly the entire assignment
 * of truth values to all atomic propositions at a particular moment.
 *
 * For explicit modeling, we reify State as a first-class entity.
 * This enables semantic reasoning about proposition extensions.
 */
sig State {}

/* ============================================================================
   TIER 3: SEMANTIC PROPOSITIONS
   ============================================================================ */

/**
 * A Proposition is an abstract entity that denotes a set of states.
 *
 * Semantic interpretation:
 *   ⟦P⟧ = { s : State | Holds[P, s] }
 *
 * More concretely, P.holds ⊆ State represents the extension of P:
 * the set of states in which P is true.
 *
 * This is fundamentally different from propositional logic where a
 * proposition is merely a named formula. Here, a proposition IS its
 * semantic extension.
 */
abstract sig Proposition {
    holds: set State
}

/**
 * Semantic satisfaction: a proposition holds in a state iff
 * the state belongs to the proposition's extension.
 */
pred Holds[p: Proposition, s: State] {
    s in p.holds
}

/* ============================================================================
   TIER 4: LOGICAL CONNECTIVES AS PROPOSITIONS
   ============================================================================ */

/**
 * Negation: ¬P
 *
 * Semantic meaning:
 *   ⟦¬P⟧ = State \ ⟦P⟧
 *
 * A negation is true in exactly those states where the operand is false.
 */
sig Not extends Proposition {
    operand: one Proposition
}

/**
 * Conjunction: P ∧ Q
 *
 * Semantic meaning:
 *   ⟦P ∧ Q⟧ = ⟦P⟧ ∩ ⟦Q⟧
 *
 * A conjunction is true where both conjuncts are true.
 */
sig And extends Proposition {
    left: one Proposition,
    right: one Proposition
}

/**
 * Disjunction: P ∨ Q
 *
 * Semantic meaning:
 *   ⟦P ∨ Q⟧ = ⟦P⟧ ∪ ⟦Q⟧
 *
 * A disjunction is true where at least one disjunct is true.
 */
sig Or extends Proposition {
    left: one Proposition,
    right: one Proposition
}

/**
 * Implication: P ⇒ Q
 *
 * Semantic meaning:
 *   ⟦P ⇒ Q⟧ = (State \ ⟦P⟧) ∪ ⟦Q⟧
 *
 * An implication is true in two cases:
 *   1. The antecedent is false (vacuous truth)
 *   2. The consequent is true
 *
 * Equivalently: ¬P ∨ Q
 */
sig Implies extends Proposition {
    antecedent: one Proposition,
    consequent: one Proposition
}

/* ============================================================================
   TIER 5: SEMANTIC MEANING OF LOGICAL OPERATORS
   ============================================================================ */

/**
 * Fact: PropositionSemantics
 *
 * This constraint defines the semantic meaning of each logical connective
 * by specifying its extension (the set of states where it holds).
 *
 * The semantics are defined inductively: the extension of a compound
 * proposition is determined from the extensions of its components.
 */
fact PropositionSemantics {

    -- Negation: ⟦¬p⟧ = State \ ⟦p⟧
    all n: Not |
        n.holds = State - n.operand.holds

    -- Conjunction: ⟦p ∧ q⟧ = ⟦p⟧ ∩ ⟦q⟧
    all a: And |
        a.holds = a.left.holds & a.right.holds

    -- Disjunction: ⟦p ∨ q⟧ = ⟦p⟧ ∪ ⟦q⟧
    all o: Or |
        o.holds = o.left.holds + o.right.holds

    -- Implication: ⟦p ⇒ q⟧ = (State \ ⟦p⟧) ∪ ⟦q⟧
    all i: Implies |
        i.holds = (State - i.antecedent.holds) + i.consequent.holds
}

/* ============================================================================
   TIER 6: FREEHAND LEMMAS
   ============================================================================ */

/**
 * A Lemma is a logical claim with the structure:
 *
 *   P₁ ∧ P₂ ∧ ... ∧ Pₙ ⇒ Q
 *
 * Semantically:
 *   ⟦P₁⟧ ∩ ⟦P₂⟧ ∩ ... ∩ ⟦Pₙ⟧ ⊆ ⟦Q⟧
 *
 * That is: every state satisfying all assumptions must satisfy the conclusion.
 */
sig Lemma {
    assumptions: set Proposition,
    conclusion: one Proposition,
    dependencies: set Lemma
}

/**
 * Predicate: AssumptionsHold[l, s]
 *
 * True iff all assumptions of lemma l hold simultaneously in state s.
 *
 *   AssumptionsHold[l, s] ⟺ ∀p ∈ l.assumptions : Holds[p, s]
 *                          ⟺ s ∈ ⋂_{p ∈ l.assumptions} ⟦p⟧
 */
pred AssumptionsHold[l: Lemma, s: State] {
    all p: l.assumptions |
        Holds[p, s]
}

/**
 * Predicate: LemmaHolds[l, s]
 *
 * The lemma holds in state s iff whenever all assumptions are satisfied,
 * the conclusion is also satisfied.
 *
 *   LemmaHolds[l, s] ⟺ AssumptionsHold[l, s] ⇒ Holds[l.conclusion, s]
 *
 * This is the semantic meaning of implication applied to lemmas.
 */
pred LemmaHolds[l: Lemma, s: State] {
    AssumptionsHold[l, s] implies Holds[l.conclusion, s]
}

/**
 * Predicate: ViolatesLemma[l, s]
 *
 * A genuine counterexample to lemma l in state s:
 * All assumptions are satisfied, but the conclusion is false.
 *
 *   ViolatesLemma[l, s] ⟺ AssumptionsHold[l, s] ∧ ¬Holds[l.conclusion, s]
 *                        ⟺ s ∈ (⋂_{p ∈ l.assumptions} ⟦p⟧) \ ⟦l.conclusion⟧
 *
 * This is a real counterexample, not vacuous. The assumptions must hold
 * for it to be meaningful.
 */
pred ViolatesLemma[l: Lemma, s: State] {
    AssumptionsHold[l, s]
    and not Holds[l.conclusion, s]
}

/**
 * Predicate: LemmaIsSound[l]
 *
 * A lemma is sound iff it holds in all reachable states.
 *
 *   LemmaIsSound[l] ⟺ ∀s : State . LemmaHolds[l, s]
 *                   ⟺ ¬∃s : State . ViolatesLemma[l, s]
 */
pred LemmaIsSound[l: Lemma] {
    all s: State |
        LemmaHolds[l, s]
}

/* ============================================================================
   TIER 7: STRUCTURAL INVARIANTS
   ============================================================================ */

/**
 * Invariant I1: Dependencies form a Directed Acyclic Graph (DAG)
 *
 * No lemma may (directly or transitively) depend on itself.
 * This ensures proof construction terminates.
 */
assert NoCyclicLemmaDependencies {
    no l: Lemma |
        l in l.^dependencies
}

/**
 * Invariant I2: Proposition Closure
 *
 * Ensures that every compound proposition's components are themselves
 * valid propositions. This is guaranteed by Alloy's type system.
 */

/**
 * Invariant I3: Lemma Well-Formedness
 *
 * Every lemma has at least a conclusion and may have assumptions.
 * (Guaranteed by signature: conclusion: one Proposition)
 */

/* ============================================================================
   TIER 8: ATOMIC PROPOSITIONS (Semantic Atoms)
   ============================================================================ */

/**
 * An atomic proposition with explicit semantic definition.
 *
 * Rather than treating InDomain as an uninterpreted atom, we define
 * its semantic extension directly via a predicate.
 */
sig InDomain extends Proposition {
    element: one Element,
    domain: one Domain
}

/**
 * Fact: Semantic meaning of InDomain
 *
 * The extension of an "InDomain" atomic proposition is the set of
 * ALL states (since domain membership is a structural property that
 * does not vary across states).
 *
 * In other words: an element either belongs to a domain (structural fact)
 * or it does not; this is invariant across all reachable states.
 */
fact InDomainSemantics {
    all p: InDomain |
        (p.element.belongsTo = p.domain) implies (p.holds = State)
        else (p.holds = none)
}

/**
 * An example atomic proposition: ElementPair
 *
 * Represents that two elements come from the same domain.
 */
sig ElementsInSameDomain extends Proposition {
    elem1: one Element,
    elem2: one Element
}

fact ElementsInSameDomainSemantics {
    all p: ElementsInSameDomain |
        (p.elem1.belongsTo = p.elem2.belongsTo)
        implies (p.holds = State)
        else (p.holds = none)
}

/* ============================================================================
   TIER 9: EXAMPLE LEMMAS
   ============================================================================ */

/**
 * EXAMPLE 1: Tautology
 *
 * Lemma: "P ∨ ¬P" (law of excluded middle)
 *
 * This should be sound (verified in all scopes).
 */
pred ExampleTautology {
    some p: Proposition, l: Lemma, or_p_not_p: Or, not_p: Not |
        not_p.operand = p and
        or_p_not_p.left = p and
        or_p_not_p.right = not_p and
        l.assumptions = none and
        l.conclusion = or_p_not_p
}

/**
 * EXAMPLE 2: Transitivity of Implication
 *
 * Lemma: (P ⇒ Q) ∧ (Q ⇒ R) ⟹ (P ⇒ R)
 *
 * This is a logical tautology and should verify.
 */
pred ExampleTransitivity {
    some p: Proposition, q: Proposition, r: Proposition,
        imp_p_q: Implies, imp_q_r: Implies, imp_p_r: Implies,
        conj: And, l: Lemma |

        imp_p_q.antecedent = p and
        imp_p_q.consequent = q and

        imp_q_r.antecedent = q and
        imp_q_r.consequent = r and

        imp_p_r.antecedent = p and
        imp_p_r.consequent = r and

        conj.left = imp_p_q and
        conj.right = imp_q_r and

        l.assumptions = {conj} and
        l.conclusion = imp_p_r
}

/**
 * EXAMPLE 3: Contradiction
 *
 * Lemma: P ∧ ¬P ⟹ Q (ex falso quodlibet)
 *
 * Any conclusion can be derived from a contradiction.
 * This should also verify (vacuously true because premise is always false).
 */
pred ExampleExFalso {
    some p: Proposition, q: Proposition, not_p: Not, conj: And, l: Lemma |
        not_p.operand = p and
        conj.left = p and
        conj.right = not_p and
        l.assumptions = {conj} and
        l.conclusion = q
}

/**
 * EXAMPLE 4: Concrete Lemma with Atomic Propositions
 *
 * Lemma: "If e1 is in domain D and e2 is in domain D, then e1 and e2 are in the same domain"
 *
 * Assumptions: InDomain(e1, D) ∧ InDomain(e2, D)
 * Conclusion: ElementsInSameDomain(e1, e2)
 */
pred ExampleConcreteTransitivity {
    some e1: Element, e2: Element, d: Domain,
        in_d1: InDomain, in_d2: InDomain,
        same_domain: ElementsInSameDomain,
        conj: And, l: Lemma |

        in_d1.element = e1 and
        in_d1.domain = d and

        in_d2.element = e2 and
        in_d2.domain = d and

        same_domain.elem1 = e1 and
        same_domain.elem2 = e2 and

        conj.left = in_d1 and
        conj.right = in_d2 and

        l.assumptions = {conj} and
        l.conclusion = same_domain
}

/* ============================================================================
   TIER 10: VERIFICATION COMMANDS
   ============================================================================ */

/**
 * SEARCH 1: Find a counterexample
 *
 * Search for a state that violates any lemma.
 * If found, the instance exhibits a concrete counterexample.
 */
run {
    some l: Lemma, s: State |
        ViolatesLemma[l, s]
} for 3 but 2 Lemma, 2 Proposition, 2 State

/**
 * SEARCH 2: Find a tautological lemma
 *
 * Search for a lemma that is sound in all states.
 */
run {
    some l: Lemma |
        LemmaIsSound[l]
} for 3 but 2 Lemma, 2 Proposition

/**
 * SEARCH 3: Construct the law of excluded middle
 */
run ExampleTautology
    for 3 but 2 Lemma, 3 Proposition, 1 State

/**
 * SEARCH 4: Construct transitivity of implication
 */
run ExampleTransitivity
    for 3 but 2 Lemma, 4 Proposition, 1 State

/**
 * SEARCH 5: Construct ex falso quodlibet
 */
run ExampleExFalso
    for 3 but 2 Lemma, 3 Proposition, 1 State

/**
 * SEARCH 6: Construct concrete domain lemma
 */
run ExampleConcreteTransitivity
    for 3 but 2 Lemma, 3 Proposition, 1 State

/* ============================================================================
   TIER 11: ASSERTIONS AND CHECKS
   ============================================================================ */

/**
 * CHECK 1: Acyclicity of lemma dependencies
 *
 * NOTE: this model has no fact forbidding cycles, so Alloy does find a
 * counterexample. invariants/core.als adds the DependencyDAG fact.
 */
check NoCyclicLemmaDependencies for 5 but 3 Lemma

/**
 * CHECK 2: Universal soundness of tautologies
 *
 * Assert that the law of excluded middle is sound in all states.
 */
assert ExcludedMiddleIsSound {
    all l: Lemma |
        (some p: Proposition, or_p_not_p: Or, not_p: Not |
            not_p.operand = p and
            or_p_not_p.left = p and
            or_p_not_p.right = not_p and
            l.assumptions = none and
            l.conclusion = or_p_not_p
        ) implies LemmaIsSound[l]
}

check ExcludedMiddleIsSound for 3 but 2 Lemma, 3 Proposition, 5 State

/**
 * CHECK 3: Contradiction implies anything (ex falso)
 *
 * If the assumptions are contradictory, the lemma is sound.
 *
 * NOTE: as written this asserts that EVERY lemma is sound, so Alloy finds
 * counterexamples at every scope below. The correctly stated invariant
 * (contradictory assumptions => sound) is invariants/lemmas.als ExFalso.
 */
assert ExFalsoIsSound {
    all l: Lemma, s: State |
        (AssumptionsHold[l, s]) implies (Holds[l.conclusion, s])
}

check ExFalsoIsSound for 4 but 2 Lemma, 3 Proposition, 5 State

/**
 * CHECK 4: Scope matrix for counterexample discovery
 */
check ExFalsoIsSound for 2 but 1 Lemma, 2 Proposition, 3 State
check ExFalsoIsSound for 4 but 2 Lemma, 3 Proposition, 5 State
check ExFalsoIsSound for 6 but 3 Lemma, 4 Proposition, 7 State
