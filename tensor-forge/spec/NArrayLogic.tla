------------------------------ MODULE NArrayLogic ------------------------------
EXTENDS Naturals, Sequences, TLC

CONSTANT N
ASSUME N \in Nat /\ N > 0

VARIABLES a, p, c, d, t, comp, r, stage, worm, sealed

Partition(x) ==
  LET k == N \div 2 IN
  [left  |-> [i \in 1..k       |-> x[i]],
   right |-> [i \in (k+1)..N   |-> x[i]]]

Product(p) ==
  LET k == N \div 2 IN
  [i \in 1..N |->
     IF i <= k THEN p.left[i] ELSE p.right[i]]

Difference(x, y) ==
  [i \in 1..N |-> x[i] - y[i]]

Transform(x) ==
  [i \in 1..N |-> x[i] + 1]

Composition(x) ==
  [i \in 1..N |-> x[i] * 2]

Closure(p) ==
  Product(p)

Invariant(x) ==
  \A i \in 1..N-1: x[i] <= x[i+1]

Init ==
  /\ a      = [i \in 1..N |-> 0]
  /\ p      = Partition(a)
  /\ c      = Product(p)
  /\ d      = Difference(a, c)
  /\ t      = Transform(d)
  /\ comp   = Composition(t)
  /\ r      = Closure(p)
  /\ stage  = "start"
  /\ worm   = <<>>
  /\ sealed = FALSE

Next ==
  \/ /\ stage = "start"
     /\ stage' = "partition"
     /\ p' = Partition(a)
     /\ UNCHANGED <<a, c, d, t, comp, r, worm, sealed>>

  \/ /\ stage = "partition"
     /\ stage' = "product"
     /\ c' = Product(p)
     /\ UNCHANGED <<a, p, d, t, comp, r, worm, sealed>>

  \/ /\ stage = "product"
     /\ stage' = "difference"
     /\ d' = Difference(a, c)
     /\ UNCHANGED <<a, p, c, t, comp, r, worm, sealed>>

  \/ /\ stage = "difference"
     /\ stage' = "transform"
     /\ t' = Transform(d)
     /\ UNCHANGED <<a, p, c, d, comp, r, worm, sealed>>

  \/ /\ stage = "transform"
     /\ stage' = "composition"
     /\ comp' = Composition(t)
     /\ UNCHANGED <<a, p, c, d, t, r, worm, sealed>>

  \/ /\ stage = "composition"
     /\ stage' = "closure"
     /\ r' = Closure(p)
     /\ UNCHANGED <<a, p, c, d, t, comp, worm, sealed>>

  \/ /\ stage = "closure"
     /\ stage' = "invariant"
     /\ a' = r
     /\ worm' = Append(worm, [op |-> "⌹☉△◇⬡○", input |-> a, output |-> r])
     /\ UNCHANGED <<p, c, d, t, comp, r, sealed>>

  \/ /\ stage = "invariant"
     /\ Invariant(a)
     /\ stage' = "sealed"
     /\ sealed' = TRUE
     /\ worm' = Append(worm, [op |-> "Ω", input |-> a, output |-> a])
     /\ UNCHANGED <<a, p, c, d, t, comp, r>>

Spec ==
  Init /\ [][Next]_<<a, p, c, d, t, comp, r, stage, worm, sealed>>
       /\ WF_<<a, p, c, d, t, comp, r, stage, worm, sealed>>(Next)

=============================================================================
