---------------- MODULE SWARM ----------------
EXTENDS Naturals, Sequences, TLC

CONSTANTS Agents, MaxLines, HASH
VARIABLES A1, A2, A3, L, State, Time, Hash, Proof,
          Verified, Committed, TemporalLock, Status

ASSUME Agents = {1, 2, 3}
ASSUME MaxLines \in Nat /\ MaxLines = 10000

(* ---------- Line-count monotonicity ---------- *)
DeltaPos == \A r \in Nat: L[r+1] > L[r]

(* ---------- Generator A1 ---------- *)
Generator(r) ==
  /\ L' = L[r] + 1
  /\ L' > L[r]
  /\ A1' = 1

(* ---------- Verifier A2 ---------- *)
Verifier(x) ==
  IF Syntax(x) /\ Type(x) /\ Invariant(x) /\ ProofOf(x)
  THEN 1 ELSE 0

Reject(x) == Verifier(x) = 0

(* ---------- Temporal arbiter A3 ---------- *)
TemporalStep(r) ==
  /\ Time[r+1] > Time[r]
  /\ State[r+1] = F(State[r], A1[r], Time[r])
  /\ A3' = 1

TemporalValid == \A r \in Nat: Time[r] < Time[r+1]

Halt3 == A3 = 0 => Status' = "HALT"

Rollback ==
  /\ A2 = 0
  /\ A1' = "ROLLBACK"
  /\ TemporalLock' = 1

Commit ==
  /\ A1 = 1 /\ A2 = 1 /\ A3 = 1
  /\ Hash[r+1] = HASH(Hash[r] \o Proof[r])
  /\ Hash[r+1] # Hash[r]
  /\ Committed' = Committed \cup {r+1}

Immutable ==
  \A r \in Nat: Committed[r] => UNCHANGED <<Hash[r], Proof[r]>>

(* ---------- Loop / Finalize / Halt ---------- *)
Loop ==
  /\ L[r] < MaxLines
  /\ A1 = 1 /\ A2 = 1 /\ A3 = 1
  /\ Status' = "LOOP"

Finalize ==
  /\ L[r] >= MaxLines
  /\ \A l \in 1..MaxLines: Verified[l] = 1
  /\ Status' = "FINALIZE"
  /\ VerifiedOutput \subseteq (MPL \cap PURE_MATH)

HaltFail ==
  /\ \E l \in 1..MaxLines: Verified[l] = 0
  /\ Status' = "HALT"

(* ---------- SWARM pipeline ---------- *)
SwarmStep(r) ==
  \/ Loop
  \/ Finalize
  \/ HaltFail
  \/ Rollback
  \/ Halt3

(* ---------- Environment ---------- *)
Init ==
  /\ A1 = 0 /\ A2 = 0 /\ A3 = 0
  /\ L = [r \in Nat |-> 0]
  /\ State = [r \in Nat |-> {}]
  /\ Time = [r \in Nat |-> 0]
  /\ Hash = [r \in Nat |-> {}]
  /\ Proof = [r \in Nat |-> {}]
  /\ Verified = [l \in 1..MaxLines |-> 0]
  /\ Committed = {}
  /\ TemporalLock = 0
  /\ Status = "INIT"

Next ==
  \E r \in Nat: SwarmStep(r)

Spec == Init /\ [][Next]_<<A1, A2, A3, L, State, Time,
                          Hash, Proof, Verified,
                          Committed, TemporalLock, Status>>

(* ---------- Liveness / safety ---------- *)
Safety ==
  /\ \A r \in Nat: Committed[r] => Hash[r+1] # Hash[r]
  /\ \A r \in Nat: Time[r] < Time[r+1]
  /\ TemporalLock = 1 => A1 = "ROLLBACK"

Liveness ==
  (L < MaxLines) ~> (L >= MaxLines)
  /\ (L >= MaxLines /\ \A l \in 1..MaxLines: Verified[l] = 1)
     ~> (Status = "FINALIZE")

ASSUME ProvableSound ==
  \A ln, pf: Provable(ln, pf) => TRUE

================================================================
