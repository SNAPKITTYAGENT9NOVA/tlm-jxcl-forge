module decomposition

/*
 * Certification invariants of matlab/+{lu,qr,svd,cholesky}/certifyDecomposition.m
 * abstracted to their boolean structure: a certificate is certified exactly
 * when every invariant its decomposition kind requires is within tolerance.
 * Lean correspondences: QR.lean (qr_reconstruction, IsOrthogonal,
 * IsUpperTriangular), LinearSolve.lean (solve_correct).
 */

abstract sig Kind {}
one sig LU, QR, SVD, Cholesky extends Kind {}

abstract sig Invariant {}
one sig Reconstruction, LowerTriangular, UpperTriangular, UnitDiagonal,
        Orthogonal, OrthogonalU, OrthogonalV, DiagonalNonneg,
        Symmetric, PositiveDiagonal, PositiveDefinite extends Invariant {}

fun required: Kind -> Invariant {
    LU -> (Reconstruction + LowerTriangular + UpperTriangular + UnitDiagonal)
  + QR -> (Reconstruction + Orthogonal + UpperTriangular)
  + SVD -> (Reconstruction + OrthogonalU + OrthogonalV + DiagonalNonneg)
  + Cholesky -> (Reconstruction + LowerTriangular + Symmetric
                 + PositiveDiagonal + PositiveDefinite)
}

sig Certificate { kind: one Kind, satisfied: set Invariant }
sig Certified in Certificate {}

fact CertificationRule {
  all c: Certificate | c in Certified iff c.kind.required in c.satisfied
}

-- D1. No certified decomposition violates a required invariant.
assert NoCertifiedViolation {
  no c: Certified | some c.kind.required - c.satisfied
}
check NoCertifiedViolation for 6 expect 0

-- D2. Reconstruction is required by every kind.
assert ReconstructionAlwaysRequired { all k: Kind | Reconstruction in k.required }
check ReconstructionAlwaysRequired for 4 expect 0

-- D3. Every kind checks between 3 and 5 invariants (README contract).
assert InvariantCountBounded { all k: Kind | #k.required >= 3 and #k.required <= 5 }
check InvariantCountBounded for 4 but 4 int expect 0

-- D4. Certified Cholesky implies a symmetric positive-definite input.
assert CholeskyNeedsSPD {
  all c: Certified | c.kind = Cholesky implies Symmetric + PositiveDefinite in c.satisfied
}
check CholeskyNeedsSPD for 6 expect 0

-- C1. Reconstruction alone is not certification (a small residual can hide
--     a non-orthogonal Q or non-triangular factor).
run ReconstructionInsufficient {
  some c: Certificate - Certified | Reconstruction in c.satisfied
} for 4 expect 1

-- C2. Dropping any single required invariant loses certification.
assert DroppingAnyInvariantDecertifies {
  all c: Certificate, i: c.kind.required | i not in c.satisfied implies c not in Certified
}
check DroppingAnyInvariantDecertifies for 6 expect 0

-- C3. LU and QR certificates are not interchangeable.
run KindMatters {
  some c: Certificate | c.kind = QR and c in Certified
    and not (LU.required in c.satisfied)
} for 4 expect 1
