---------------- MODULE SWARM ----------------
EXTENDS Naturals, Sequences, TLC

(* A three-agent generate/verify/temporal-commit loop over `MaxLines`
   candidate lines. `GoodLines` is the model's stand-in for the real
   Syntax/Type/Invariant/ProofOf verifier: the subset of `1..MaxLines`
   that would pass review. Every variable holds the *current* value
   (round-indexed via `line`/`Len(hashTrail)`, not stored as an
   unbounded function from Nat), so the state space is finite and
   `TLC` can actually explore it. *)

CONSTANTS MaxLines, GoodLines
VARIABLES line, a1, a2, a3, time, hashTrail, verified, committed,
          temporalLock, status

ASSUME MaxLines \in Nat /\ MaxLines > 0
ASSUME GoodLines \subseteq 1..MaxLines

Hash(prev, ln) == prev + ln + 1

vars == <<line, a1, a2, a3, time, hashTrail, verified, committed,
          temporalLock, status>>

Terminal == status \in {"FINALIZE", "HALT"}

Init ==
  /\ line = 0
  /\ a1 = "IDLE"
  /\ a2 = 0
  /\ a3 = 0
  /\ time = 0
  /\ hashTrail = <<0>>
  /\ verified = [l \in 1..MaxLines |-> FALSE]
  /\ committed = {}
  /\ temporalLock = 0
  /\ status = "GENERATE"

(* A1: generator. Advances the line counter and raises its own flag;
   blocked while `temporalLock` is set (a prior Rollback halted the
   pipeline). *)
GenerateStep ==
  /\ status = "GENERATE"
  /\ temporalLock = 0
  /\ line' = line + 1
  /\ a1' = "RAISED"
  /\ status' = "VERIFY"
  /\ UNCHANGED <<a2, a3, time, hashTrail, verified, committed, temporalLock>>

(* A2: verifier. `line \in GoodLines` stands in for
   `Syntax /\ Type /\ Invariant /\ ProofOf` all holding for this line. *)
VerifyStep ==
  /\ status = "VERIFY"
  /\ a2' = IF line \in GoodLines THEN 1 ELSE 0
  /\ status' = IF line \in GoodLines THEN "TEMPORAL" ELSE "ROLLBACK"
  /\ UNCHANGED <<line, a1, a3, time, hashTrail, verified, committed, temporalLock>>

(* A3: temporal arbiter. Strictly advances `time`. *)
TemporalStep ==
  /\ status = "TEMPORAL"
  /\ a2 = 1
  /\ time' = time + 1
  /\ a3' = 1
  /\ status' = "COMMIT"
  /\ UNCHANGED <<line, a1, a2, hashTrail, verified, committed, temporalLock>>

(* A2 rejected: roll back and latch the temporal lock, halting the
   pipeline for good (matches the boxed safety clause
   `TemporalLock = 1 => A1 = "ROLLBACK"`). *)
RollbackStep ==
  /\ status = "ROLLBACK"
  /\ a2 = 0
  /\ a1' = "ROLLBACK"
  /\ temporalLock' = 1
  /\ status' = "HALT"
  /\ UNCHANGED <<line, a2, a3, time, hashTrail, verified, committed>>

(* All three agents agree: chain the hash, mark the line verified and
   committed, reset the per-round flags, and either loop back for the
   next line or move on to the final check. *)
CommitStep ==
  /\ status = "COMMIT"
  /\ a1 = "RAISED" /\ a2 = 1 /\ a3 = 1
  /\ hashTrail' = Append(hashTrail, Hash(hashTrail[Len(hashTrail)], line))
  /\ verified' = [verified EXCEPT ![line] = TRUE]
  /\ committed' = committed \cup {line}
  /\ a1' = "IDLE"
  /\ a2' = 0
  /\ a3' = 0
  /\ status' = IF line >= MaxLines THEN "CHECK" ELSE "GENERATE"
  /\ UNCHANGED <<line, time, temporalLock>>

FinalizeStep ==
  /\ status = "CHECK"
  /\ \A l \in 1..MaxLines: verified[l]
  /\ status' = "FINALIZE"
  /\ UNCHANGED <<line, a1, a2, a3, time, hashTrail, verified, committed, temporalLock>>

HaltFailStep ==
  /\ status = "CHECK"
  /\ \E l \in 1..MaxLines: ~verified[l]
  /\ status' = "HALT"
  /\ UNCHANGED <<line, a1, a2, a3, time, hashTrail, verified, committed, temporalLock>>

RealNext ==
  \/ GenerateStep
  \/ VerifyStep
  \/ TemporalStep
  \/ RollbackStep
  \/ CommitStep
  \/ FinalizeStep
  \/ HaltFailStep

(* Absorbing self-loop once terminal; kept out of `RealNext` (and so
   out of the `WF_vars(RealNext)` fairness obligation below) for the
   same reason as in NArrayLogic.tla: a fairness requirement on an
   `UNCHANGED vars` action can never be honestly satisfied once it is
   the only action left enabled. *)
Next ==
  \/ RealNext
  \/ /\ Terminal
     /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars /\ WF_vars(RealNext)

TypeOK ==
  /\ line \in 0..MaxLines
  /\ a1 \in {"IDLE", "RAISED", "ROLLBACK"}
  /\ a2 \in {0, 1}
  /\ a3 \in {0, 1}
  /\ time \in Nat
  /\ hashTrail \in Seq(Nat)
  /\ Len(hashTrail) >= 1
  /\ verified \in [1..MaxLines -> BOOLEAN]
  /\ committed \subseteq 1..MaxLines
  /\ temporalLock \in {0, 1}
  /\ status \in
       {"GENERATE", "VERIFY", "TEMPORAL", "ROLLBACK", "COMMIT",
        "CHECK", "FINALIZE", "HALT"}

(* The two safety clauses the boxed spec calls out by name. *)
TemporalMonotone == \A i \in 1..(Len(hashTrail) - 1): hashTrail[i] < hashTrail[i + 1]
TemporalLockImpliesRollback == temporalLock = 1 => a1 = "ROLLBACK"
CommittedImpliesVerified == committed \subseteq {l \in 1..MaxLines: verified[l]}

Safety == TemporalMonotone /\ TemporalLockImpliesRollback /\ CommittedImpliesVerified

EventuallyTerminal == <>Terminal

================================================================
