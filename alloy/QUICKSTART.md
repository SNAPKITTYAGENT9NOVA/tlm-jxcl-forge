# Freehand Lemmas in Alloy — Quick Start

## Files

- **FreehandLemmas.als** — Complete Alloy specification with examples
- **FreehandLemmas-Semantics.md** — Comprehensive semantic documentation
- **QUICKSTART.md** — This file

## Key Insight

Each **Proposition** denotes a set of States:

```
⟦P⟧ = P.holds ⊆ State
```

## Running Examples

### 1. Find a Tautology

```
run ExampleTautology for 3
```

Constructs the law of excluded middle (P ∨ ¬P) and verifies it's always true.

### 2. Find a Counterexample

```
run {
    some l: Lemma, s: State |
        ViolatesLemma[l, s]
} for 3 but 2 Lemma
```

Searches for any lemma with a violating state.

### 3. Verify Transitivity

```
run ExampleTransitivity for 3
```

Constructs (P ⇒ Q) ∧ (Q ⇒ R) ⇒ (P ⇒ R) and verifies it holds.

### 4. Check Acyclicity

```
check NoCyclicLemmaDependencies for 5
```

Ensures lemmas don't form circular dependencies.

## Creating Custom Propositions

**Atomic Proposition Template:**

```alloy
sig MyProposition extends Proposition {
    element: one Element,
    property: String
}

fact MyPropositionSemantics {
    all p: MyProposition |
        (HasProperty[p.element, p.property])
        implies (p.holds = State)
        else (p.holds = none)
}
```

## Creating Lemmas

**Pattern:**

```alloy
pred MyLemmaExample {
    some p: Proposition, q: Proposition, l: Lemma |
        -- Define assumptions
        l.assumptions = {p} and
        -- Define conclusion
        l.conclusion = q
}
```

## Interpreting Results

| Alloy Says | Meaning |
|-----------|---------|
| Instance found | A satisfying state exists |
| No instance | No violation found (within scope) |
| Contradiction | Unsatisfiable formula |

## Core Predicates

| Predicate | Meaning |
|-----------|---------|
| `Holds[p, s]` | Proposition p is true in state s |
| `AssumptionsHold[l, s]` | All assumptions of lemma l hold in s |
| `LemmaHolds[l, s]` | Lemma l is satisfied in s |
| `ViolatesLemma[l, s]` | State s is a counterexample to lemma l |
| `LemmaIsSound[l]` | Lemma l holds in all states |

## Semantic Operations

| Operation | Alloy Syntax | Semantics |
|-----------|---|---------|
| Negation | `Not[p]` | ⟦¬P⟧ = State \ ⟦P⟧ |
| Conjunction | `And[p, q]` | ⟦P ∧ Q⟧ = ⟦P⟧ ∩ ⟦Q⟧ |
| Disjunction | `Or[p, q]` | ⟦P ∨ Q⟧ = ⟦P⟧ ∪ ⟦Q⟧ |
| Implication | `Implies[p, q]` | ⟦P ⇒ Q⟧ = (State \ ⟦P⟧) ∪ ⟦Q⟧ |

## Scope Recommendations

```alloy
-- Minimal test
check AllLemmasSound for 2

-- Small examples
run ExampleTautology for 3 but 2 Lemma, 3 Proposition

-- Medium verification
check NoCyclicLemmaDependencies for 5 but 3 Lemma

-- Exhaustive within scope
run { some l: Lemma | LemmaIsSound[l] } for 6 but 4 Lemma
```

## Common Patterns

### Check if Lemma is Sound

```alloy
check {
    all l: Lemma |
        LemmaIsSound[l]
} for 5
```

### Search for Counterexamples

```alloy
run {
    some l: Lemma, s: State |
        ViolatesLemma[l, s]
} for 4 but 2 Lemma
```

### Verify Acyclicity

```alloy
check {
    no l: Lemma | l in l.^dependencies
} for 5
```

### Construct a Tautology

```alloy
run {
    some l: Lemma, p: Proposition |
        l.assumptions = none and
        l.conclusion = p and
        p.holds = State  -- Always true
} for 3
```

## Next Steps

1. **Read FreehandLemmas-Semantics.md** for complete semantic definitions
2. **Study examples** in FreehandLemmas.als (Tiers 9-11)
3. **Create custom atomic propositions** (Tier 8 pattern)
4. **Build domain-specific lemmas** using the framework
5. **Run verification** at increasing scopes to explore behavior

## Tips

- **Start small:** Use scope 3 or 4 for initial exploration
- **Bound signatures:** Use `for N but M Lemma, K Proposition` to control search space
- **Examine instances:** Look at proposition.holds to understand truth assignments
- **Increase scope gradually:** If no counterexample at 3, try 5, then 7
- **Use predicates:** Custom `pred` blocks make lemma construction cleaner

## Common Pitfalls

❌ **Mistake:** Assuming no counterexample = universal proof
✅ **Correct:** No counterexample = consistent within bounded scope

❌ **Mistake:** Ignoring scope in conclusions
✅ **Correct:** Always state "verified for scope N atoms"

❌ **Mistake:** Building lemmas with arbitrary assumptions
✅ **Correct:** Ground assumptions in domain predicates (atomic propositions)

❌ **Mistake:** Allowing circular dependencies
✅ **Correct:** Verify `NoCyclicLemmaDependencies` assertion holds

## References

- **Full Specification:** FreehandLemmas.als (11 tiers, 300+ lines)
- **Semantics:** FreehandLemmas-Semantics.md (complete formal treatment)
- **Alloy Documentation:** http://alloy.mit.edu

---

**Status:** Ready to use. Load FreehandLemmas.als into Alloy Analyzer 6.0+ and execute commands from the menu.
