module core

/*
 * Hardened semantic core shared by every invariant module.
 *
 * Differences from ../FreehandLemmas.als, each closing a counterexample
 * the original model admits:
 *   H1  Propositions are well-founded (no p is its own sub-formula);
 *       otherwise Not.holds = State - Not.holds is only satisfiable
 *       with an empty State set, silently shrinking the model.
 *   H2  Lemma dependencies form a DAG as a fact, not only an assertion
 *       (the original NoCyclicLemmaDependencies check is SAT).
 *   H3  Atom gives a free, uninterpreted extension so propositional
 *       laws are checked over every possible valuation.
 */

sig Domain {}
sig Element { belongsTo: one Domain }
sig State {}

abstract sig Proposition { holds: set State }

sig Atom extends Proposition {}
sig Not extends Proposition { operand: one Proposition }
sig And extends Proposition { left: one Proposition, right: one Proposition }
sig Or extends Proposition { left: one Proposition, right: one Proposition }
sig Implies extends Proposition { antecedent: one Proposition, consequent: one Proposition }

sig InDomain extends Proposition { element: one Element, domain: one Domain }
sig ElementsInSameDomain extends Proposition { elem1: one Element, elem2: one Element }

fun subformula: Proposition -> Proposition {
  operand + (And <: left) + (And <: right) + (Or <: left) + (Or <: right)
    + antecedent + consequent
}

fact WellFounded { no p: Proposition | p in p.^subformula }

fact Semantics {
  all n: Not | n.holds = State - n.operand.holds
  all a: And | a.holds = a.left.holds & a.right.holds
  all o: Or | o.holds = o.left.holds + o.right.holds
  all i: Implies | i.holds = (State - i.antecedent.holds) + i.consequent.holds
  all p: InDomain |
    p.holds = (p.element.belongsTo = p.domain implies State else none)
  all p: ElementsInSameDomain |
    p.holds = (p.elem1.belongsTo = p.elem2.belongsTo implies State else none)
}

sig Lemma {
  assumptions: set Proposition,
  conclusion: one Proposition,
  dependencies: set Lemma
}

fact DependencyDAG { no l: Lemma | l in l.^dependencies }

pred Holds[p: Proposition, s: State] { s in p.holds }
pred AssumptionsHold[l: Lemma, s: State] { all p: l.assumptions | Holds[p, s] }
pred LemmaHolds[l: Lemma, s: State] { AssumptionsHold[l, s] implies Holds[l.conclusion, s] }
pred ViolatesLemma[l: Lemma, s: State] { AssumptionsHold[l, s] and not Holds[l.conclusion, s] }
pred LemmaIsSound[l: Lemma] { all s: State | LemmaHolds[l, s] }
pred Contradictory[l: Lemma] { no s: State | AssumptionsHold[l, s] }
