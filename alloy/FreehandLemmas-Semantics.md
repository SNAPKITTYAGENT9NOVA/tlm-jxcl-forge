# Freehand Lemmas: State-Based Semantic Framework

## Overview

This document explains the refined **state-based semantic framework** for Freehand Lemmas in Alloy, where propositions denote actual sets of model states rather than being uninterpreted atoms.

### Key Innovation

**Before (Uninterpreted):**
```
sig Proposition { name: String }
```
A proposition is merely a label with no inherent meaning.

**After (Semantic):**
```
abstract sig Proposition { holds: set State }
```
A proposition IS its semantic extension: the set of states where it is true.

---

## Table of Contents

1. [Semantic Framework](#semantic-framework)
2. [Propositions as State Sets](#propositions-as-state-sets)
3. [Logical Connectives](#logical-connectives)
4. [Lemma Semantics](#lemma-semantics)
5. [Atomic Propositions](#atomic-propositions)
6. [Examples](#examples)
7. [Verification Strategy](#verification-strategy)
8. [Formal Semantics](#formal-semantics)

---

## Semantic Framework

### Core Insight

In classical mathematical logic:
- A proposition P is a formula (syntax)
- A model M assigns truth values to propositions
- ⟦P⟧_M denotes the truth value of P in model M

In our Alloy framework:
- A State represents a complete model assignment
- A Proposition's extension `p.holds` is the set of states where P is true
- Holds[p, s] means s ∈ p.holds

### Tier Structure

```
┌─────────────────────────────┐
│ TIER 11: Checks & Assertions│
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 10: Verification Cmds  │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 9: Example Lemmas      │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 8: Atomic Propositions │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 7: Structural Invariants
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 6: Freehand Lemmas     │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 5: Semantic Meaning    │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 4: Logical Connectives │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 3: Semantic Propositions
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 2: State Space         │
└─────────────────────────────┘
             ↓
┌─────────────────────────────┐
│ TIER 1: Semantic Domain     │
└─────────────────────────────┘
```

---

## Propositions as State Sets

### Definition

A **Proposition** is an abstract entity characterized by:

```alloy
abstract sig Proposition {
    holds: set State
}
```

**Semantic interpretation:**
```
⟦P⟧ = P.holds ⊆ State

Holds[P, s] ⟺ s ∈ P.holds
```

### Extensional Equality

Two propositions are semantically equal iff they have identical extensions:

```
P ≡ Q  ⟺  P.holds = Q.holds
```

This differs from syntactic equality (P = Q, which is structural).

### State Space

```alloy
sig State {}
```

A State represents a complete assignment of truth to all atomic facts about the domain model. In Alloy's semantics, each satisfying instance corresponds to one State.

**Example:** If we have elements {e1, e2, e3} and domain {D1, D2}, a State might specify:
- e1 ∈ D1, e2 ∈ D1, e3 ∈ D2
- (other properties determined by Alloy's full instance)

---

## Logical Connectives

### 1. Negation: ¬P (Not)

```alloy
sig Not extends Proposition {
    operand: one Proposition
}

fact PropositionSemantics {
    all n: Not |
        n.holds = State - n.operand.holds
}
```

**Semantic definition:**
```
⟦¬P⟧ = State \ ⟦P⟧
```

**Interpretation:** A negation is true in exactly those states where the operand is false.

**Example:**
```
P.holds = {s1, s2, s3}
(¬P).holds = {s4, s5, ..., sn}  (all other states)
```

### 2. Conjunction: P ∧ Q (And)

```alloy
sig And extends Proposition {
    left: one Proposition,
    right: one Proposition
}

fact PropositionSemantics {
    all a: And |
        a.holds = a.left.holds & a.right.holds
}
```

**Semantic definition:**
```
⟦P ∧ Q⟧ = ⟦P⟧ ∩ ⟦Q⟧
```

**Interpretation:** A conjunction is true where both conjuncts hold.

**Example:**
```
P.holds = {s1, s2, s3}
Q.holds = {s2, s3, s4}
(P ∧ Q).holds = {s2, s3}  (intersection)
```

### 3. Disjunction: P ∨ Q (Or)

```alloy
sig Or extends Proposition {
    left: one Proposition,
    right: one Proposition
}

fact PropositionSemantics {
    all o: Or |
        o.holds = o.left.holds + o.right.holds
}
```

**Semantic definition:**
```
⟦P ∨ Q⟧ = ⟦P⟧ ∪ ⟦Q⟧
```

**Interpretation:** A disjunction is true where at least one disjunct holds.

**Example:**
```
P.holds = {s1, s2}
Q.holds = {s2, s3}
(P ∨ Q).holds = {s1, s2, s3}  (union)
```

### 4. Implication: P ⇒ Q (Implies)

```alloy
sig Implies extends Proposition {
    antecedent: one Proposition,
    consequent: one Proposition
}

fact PropositionSemantics {
    all i: Implies |
        i.holds = (State - i.antecedent.holds) + i.consequent.holds
}
```

**Semantic definition:**
```
⟦P ⇒ Q⟧ = (State \ ⟦P⟧) ∪ ⟦Q⟧
           = ⟦¬P ∨ Q⟧
```

**Interpretation:** An implication is true where:
1. The antecedent is false (vacuous truth), OR
2. The consequent is true

**Example:**
```
P.holds = {s1, s2}
Q.holds = {s2, s3}
(P ⇒ Q).holds = {s3, s4, ..., sn, s2}
              = (all states except s1)
```

---

## Lemma Semantics

### Structure

A **Lemma** encodes an implicative claim:

```alloy
sig Lemma {
    assumptions: set Proposition,
    conclusion: one Proposition,
    dependencies: set Lemma
}
```

**Logical form:**
```
P₁ ∧ P₂ ∧ ... ∧ Pₙ ⇒ Q

where:
  assumptions = {P₁, P₂, ..., Pₙ}
  conclusion = Q
```

### Satisfaction Predicates

#### 1. AssumptionsHold[l, s]

```alloy
pred AssumptionsHold[l: Lemma, s: State] {
    all p: l.assumptions |
        Holds[p, s]
}
```

**Meaning:** All assumptions are simultaneously true in state s.

```
AssumptionsHold[l, s] ⟺ s ∈ ⋂_{P ∈ l.assumptions} ⟦P⟧
```

**Vacuous case:** If l.assumptions is empty, AssumptionsHold[l, s] is true for all s.

#### 2. LemmaHolds[l, s]

```alloy
pred LemmaHolds[l: Lemma, s: State] {
    AssumptionsHold[l, s] implies Holds[l.conclusion, s]
}
```

**Meaning:** Whenever all assumptions hold, the conclusion must hold.

This is the definition of implication applied to lemmas.

#### 3. ViolatesLemma[l, s]

```alloy
pred ViolatesLemma[l: Lemma, s: State] {
    AssumptionsHold[l, s]
    and not Holds[l.conclusion, s]
}
```

**Meaning:** A genuine counterexample where:
- All assumptions are satisfied, AND
- The conclusion is false

This is **NOT** a vacuous counterexample (which would have false assumptions).

**Semantic characterization:**
```
ViolatesLemma[l, s] ⟺ s ∈ (⋂_{P ∈ l.assumptions} ⟦P⟧) \ ⟦l.conclusion⟧
```

#### 4. LemmaIsSound[l]

```alloy
pred LemmaIsSound[l: Lemma] {
    all s: State |
        LemmaHolds[l, s]
}
```

**Meaning:** A lemma is sound iff no counterexample exists.

```
LemmaIsSound[l] ⟺ ¬∃s : ViolatesLemma[l, s]
                 ⟺ ∀s : LemmaHolds[l, s]
```

---

## Atomic Propositions

### Defining Atomic Truth

Rather than allowing arbitrary truth assignments, atomic propositions should be defined by predicates over the domain model.

#### Example 1: InDomain

```alloy
sig InDomain extends Proposition {
    element: one Element,
    domain: one Domain
}

fact InDomainSemantics {
    all p: InDomain |
        (p.element.belongsTo = p.domain) implies (p.holds = State)
        else (p.holds = none)
}
```

**Interpretation:**
- If the element belongs to the domain, the proposition is true in ALL states
- If not, the proposition is false in ALL states

**Why:** Domain membership is a structural property that doesn't vary across instances.

#### Example 2: ElementsInSameDomain

```alloy
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
```

**Pattern:** Atomic propositions are defined by structural constraints on the domain model.

### General Pattern

```alloy
sig MyProposition extends Proposition {
    -- Fields encoding the proposition's parameters
    param1: one Foo,
    param2: one Bar
}

fact MyPropositionSemantics {
    all p: MyProposition |
        -- Define when p is true (in all states or specific states)
        (SomeCondition[p.param1, p.param2])
        implies (p.holds = State)  -- Always true
        else (p.holds = none)       -- Never true
}
```

---

## Examples

### Example 1: Law of Excluded Middle

**Claim:** For any proposition P, either P or ¬P must be true.

**Formal representation:**
```alloy
pred ExampleTautology {
    some p: Proposition, l: Lemma, or_p_not_p: Or, not_p: Not |
        not_p.operand = p and
        or_p_not_p.left = p and
        or_p_not_p.right = not_p and
        l.assumptions = none and
        l.conclusion = or_p_not_p
}
```

**Semantic verification:**

For any state s and proposition P:
```
s ∈ ⟦P ∨ ¬P⟧
  = ⟦P⟧ ∪ ⟦¬P⟧
  = ⟦P⟧ ∪ (State \ ⟦P⟧)
  = State
```

Therefore: (P ∨ ¬P).holds = State (always true)

**Alloy Outcome:** No counterexample found (tautology verified)

### Example 2: Transitivity of Implication

**Claim:** If (P ⇒ Q) and (Q ⇒ R), then (P ⇒ R)

**Formal representation:**
```alloy
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
```

**Semantic verification:**

Assume (P ⇒ Q) and (Q ⇒ R) hold in state s. We must show (P ⇒ R) holds in s.

Case 1: P is false in s
→ (P ⇒ R) is true in s (vacuously)

Case 2: P is true in s
→ Since (P ⇒ Q) holds and P is true, Q must be true
→ Since (Q ⇒ R) holds and Q is true, R must be true
→ (P ⇒ R) is true in s

Therefore: The lemma is sound (no counterexample).

**Alloy Outcome:** No counterexample found (logical tautology)

### Example 3: Concrete Lemma with Atomic Propositions

**Claim:** If e1 and e2 are both in domain D, then they are in the same domain.

**Formal representation:**
```alloy
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
```

**Semantic verification:**

By InDomainSemantics:
- in_d1.holds = (e1.belongsTo = d) ? State : none
- in_d2.holds = (e2.belongsTo = d) ? State : none

By ElementsInSameDomainSemantics:
- same_domain.holds = (e1.belongsTo = e2.belongsTo) ? State : none

If assumptions hold:
- e1.belongsTo = d
- e2.belongsTo = d
→ e1.belongsTo = e2.belongsTo
→ same_domain.holds = State ✓

Therefore: No counterexample (lemma is sound)

**Alloy Outcome:** Verification succeeds

---

## Verification Strategy

### Bounded vs. Unbounded Verification

**Bounded (Alloy):**
```
check ExFalsoIsSound for 4 but 2 Lemma, 3 Proposition, 5 State
```

Searches instances with ≤ 4 atoms per signature (except explicitly bounded ones).

**Meaning:** No counterexample exists within this bounded state space.

### Scope Matrix

| Scope | Use Case | Typical Finding |
|-------|----------|-----------------|
| 2 | Minimal test | Trivial lemmas |
| 4 | Small examples | Basic logic |
| 6 | Medium complexity | Compound propositions |
| 8+ | Exhaustive search | Complex interactions |

### Interpretation of Results

| Alloy Result | Semantic Conclusion | Next Step |
|--------------|-------------------|-----------|
| UNSAT (no counterexample) | Lemma is consistent at this scope | Increase scope or accept result |
| SAT (counterexample found) | Lemma is FALSE at this scope | Examine counterexample; refine lemma |
| Instance found (tautology) | Lemma is universally sound | Proof by tautology |

### Critical Caveat

**No matter the scope:**
- ✅ Counterexample DISPROVES the lemma (within scope)
- ❌ No counterexample does NOT prove the lemma universally
- ⚠️  Need external proof for unbounded universal truth

---

## Formal Semantics

### Denotational Semantics

For any Alloy instance I:

```
⟦P⟧_I : Proposition → ℘(State)

⟦P⟧_I = P.holds
```

### Semantic Meaning of Connectives

| Connective | Syntax | Semantics |
|------------|--------|-----------|
| Negation | Not[p] | ⟦¬P⟧ = State \ ⟦P⟧ |
| Conjunction | And[l, r] | ⟦P ∧ Q⟧ = ⟦P⟧ ∩ ⟦Q⟧ |
| Disjunction | Or[l, r] | ⟦P ∨ Q⟧ = ⟦P⟧ ∪ ⟦Q⟧ |
| Implication | Implies[a, c] | ⟦P ⇒ Q⟧ = (State \ ⟦P⟧) ∪ ⟦Q⟧ |

### Lemma Semantics

```
⟦Lemma(A₁,...,Aₙ,C)⟧ = ⟦A₁⟧ ∩ ... ∩ ⟦Aₙ⟧ ⊆ ⟦C⟧

Equivalently:
  ∀s ∈ State : (s ∈ ⟦A₁⟧ ∧ ... ∧ s ∈ ⟦Aₙ⟧) ⇒ (s ∈ ⟦C⟧)
```

### Counterexample

```
∃s ∈ State : (s ∈ ⟦A₁⟧ ∧ ... ∧ s ∈ ⟦Aₙ⟧) ∧ (s ∉ ⟦C⟧)
```

---

## Comparison: Before vs. After

### Uninterpreted Framework (Before)

```alloy
sig Proposition { name: String }

pred FreehandLemma[l: Lemma] {
    FreehandLemma[l]  -- Circular; no semantic meaning
}
```

**Problem:** Propositions are uninterpreted names with no inherent truth conditions.

### State-Based Semantic Framework (After)

```alloy
abstract sig Proposition { holds: set State }

fact PropositionSemantics {
    all n: Not | n.holds = State - n.operand.holds
    all a: And | a.holds = a.left.holds & a.right.holds
    all o: Or | o.holds = o.left.holds + o.right.holds
    all i: Implies | i.holds = (State - i.antecedent.holds) + i.consequent.holds
}

pred ViolatesLemma[l: Lemma, s: State] {
    AssumptionsHold[l, s]
    and not Holds[l.conclusion, s]
}
```

**Advantages:**
- ✅ Propositions have explicit truth conditions
- ✅ Connectives have well-defined semantics
- ✅ Counterexamples are concrete (a specific state s)
- ✅ Lemma satisfaction is provably decidable within scope
- ✅ Atomic propositions inherit meaning from domain predicates

---

## Usage Guide

### Running the Framework

In Alloy Analyzer:

1. **Load the specification:** File → Open → FreehandLemmas.als
2. **Select a command:** Show menu with available run/check commands
3. **Execute:** Press "Execute" (or Ctrl+E)
4. **Examine results:**
   - Instance found? Inspect proposition extensions and state valuations
   - No instance? Lemma is consistent at this scope

### Creating Custom Atomic Propositions

**Template:**

```alloy
sig MyProposition extends Proposition {
    param1: one EntityType1,
    param2: one EntityType2
}

fact MyPropositionSemantics {
    all p: MyProposition |
        (Condition[p.param1, p.param2])
        implies (p.holds = State)
        else (p.holds = none)
}
```

**Example:** "e1 and e2 are related"

```alloy
sig Related extends Proposition {
    elem1: one Element,
    elem2: one Element
}

fact RelatedSemantics {
    all r: Related |
        (some c: Connection | c.source = r.elem1 and c.target = r.elem2)
        implies (r.holds = State)
        else (r.holds = none)
}
```

### Building Lemmas

**Template:**

```alloy
some l: Lemma |
    (some p1, p2, c: Proposition |
        l.assumptions = {p1, p2} and
        l.conclusion = c
    )
```

**Example:** If P₁ and P₂ hold, then their conjunction must hold

```alloy
run {
    some l: Lemma, p1, p2, conj: And, s: State |
        conj.left = p1 and
        conj.right = p2 and
        l.assumptions = {p1, p2} and
        l.conclusion = conj and
        AssumptionsHold[l, s] and
        Holds[l.conclusion, s]
} for 3 but 2 Lemma, 3 Proposition, 2 State
```

---

## References

- Jackson, D. *Software Abstractions* (2nd ed.). MIT Press, 2012.
- Alloy Project: http://alloy.mit.edu
- Tarski, A. *The Semantic Conception of Truth*. Philosophy of Science, 1944.
