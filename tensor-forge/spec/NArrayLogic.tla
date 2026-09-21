------------------------------ MODULE NArrayLogic ------------------------------
EXTENDS Naturals, Sequences, TLC

CONSTANT N
ASSUME N \in Nat /\ N > 0

VARIABLES a, p, c, d, t, comp, r, stage, worm, sealed

Partition(x) ==
  LET k == N \div 2 IN
  [left  |-> [i \in 1..k       |-> x[i]],
   right |-> [i \in (k+1)..N   |-> x[i]]]

Product(pp) ==
  LET k == N \div 2 IN
  [i \in 1..N |->
     IF i <= k THEN pp.left[i] ELSE pp.right[i]]

Difference(x, y) ==
  [i \in 1..N |-> IF x[i] >= y[i] THEN x[i] - y[i] ELSE 0]

Transform(x) ==
  [i \in 1..N |-> x[i] + 1]

Composition(x) ==
  [i \in 1..N |-> x[i] * 2]

Closure(pp) ==
  Product(pp)

Invariant(x) ==
  \A i \in 1..N-1: x[i] <= x[i+1]

vars == <<a, p, c, d, t, comp, r, stage, worm, sealed>>

Terminal == stage \in {"sealed", "failed"}

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

RealNext ==
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
     /\ worm' = Append(worm, [op |-> "close", input |-> a, output |-> r])
     /\ UNCHANGED <<p, c, d, t, comp, r, sealed>>

  \/ /\ stage = "invariant"
     /\ Invariant(a)
     /\ stage' = "sealed"
     /\ sealed' = TRUE
     /\ worm' = Append(worm, [op |-> "seal", input |-> a, output |-> a])
     /\ UNCHANGED <<a, p, c, d, t, comp, r>>

  \/ /\ stage = "invariant"
     /\ ~Invariant(a)
     /\ stage' = "failed"
     /\ worm' = Append(worm, [op |-> "fail", input |-> a, output |-> a])
     /\ UNCHANGED <<a, p, c, d, t, comp, r, sealed>>

  \* Absorbing self-loop once terminal, so the machine is total: a
  \* terminal `stage` is a resting state, not a genuine TLC deadlock.
  \* Kept out of `RealNext` (and so out of the `WF_vars(RealNext)`
  \* fairness obligation below): a fairness requirement on an action
  \* whose own effect is `UNCHANGED vars` can never be honestly
  \* satisfied once it is the only action left enabled.
Next ==
  \/ RealNext
  \/ /\ Terminal
     /\ UNCHANGED vars

ArrayType == [1..N -> Nat]

TypeOK ==
  /\ a \in ArrayType
  /\ p \in [left: [1..(N \div 2) -> Nat], right: [(N \div 2 + 1)..N -> Nat]]
  /\ c \in ArrayType
  /\ d \in ArrayType
  /\ t \in ArrayType
  /\ comp \in ArrayType
  /\ r \in ArrayType
  /\ stage \in
       {"start", "partition", "product", "difference", "transform",
        "composition", "closure", "invariant", "sealed", "failed"}
  /\ sealed \in BOOLEAN
  /\ worm \in Seq([op: STRING, input: ArrayType, output: ArrayType])

EventuallyTerminal == <>Terminal

Spec == Init /\ [][Next]_vars /\ WF_vars(RealNext)

=============================================================================
