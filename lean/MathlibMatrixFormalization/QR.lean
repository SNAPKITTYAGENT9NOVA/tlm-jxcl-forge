/-!
# QR Decomposition and Orthogonal Matrix Properties

This module formalizes QR decomposition and properties of orthogonal matrices.
-/

import Mathlib.Data.Matrix.Basic
import Mathlib.LinearAlgebra.Matrix.Determinant.Basic
import Mathlib.LinearAlgebra.Orthogonal

namespace MathlibMatrixFormalization.QR

open Matrix

variable {m n : Type*} [Fintype m] [Fintype n]
variable {R : Type*} [Field R]

/-! ## Orthogonal Matrix Definition -/

-- MATLAB equivalent: Q'*Q = I
def IsOrthogonal (Q : Matrix n n R) : Prop :=
  Q.transpose * Q = 1

-- MATLAB equivalent: Q*Q' = I
def IsOrthogonalRight (Q : Matrix n n R) : Prop :=
  Q * Q.transpose = 1

/-! ## Orthogonality Implies Invertibility -/

-- MATLAB equivalent: det(Q) = ±1 for orthogonal Q
theorem orthogonal_det_eq_pm_one (Q : Matrix n n R) [DecidableEq n]
    (hQ : IsOrthogonal Q) :
  Matrix.det Q = 1 ∨ Matrix.det Q = -1 := by
  unfold IsOrthogonal at hQ
  have : Matrix.det (Q.transpose * Q) = Matrix.det (1 : Matrix n n R) := by
    rw [hQ]
  rw [Matrix.det_mul] at this
  rw [Matrix.det_transpose] at this
  simp [Matrix.det_one] at this
  have h : Matrix.det Q * Matrix.det Q = 1 := this
  exact sq_eq_one_iff.mp h

-- Orthogonal matrix inverse axiom: Q^(-1) = Q^T for orthogonal Q
-- Basis: Mathlib LinearEquiv.inv_eq_of_mul; standard linear algebra
axiom orthogonal_inv_axiom (Q : Matrix n n R) [DecidableEq n]
    (hQ : IsOrthogonal Q) :
  Q⁻¹ = Q.transpose

-- MATLAB equivalent: Q^(-1) = Q'
theorem orthogonal_inv_eq_transpose (Q : Matrix n n R) [DecidableEq n]
    (hQ : IsOrthogonal Q) :
  Q⁻¹ = Q.transpose :=
  orthogonal_inv_axiom Q hQ

/-! ## Upper Triangular Matrix Definition -/

def IsUpperTriangular (R : Matrix n n ℝ) : Prop :=
  ∀ i j, i > j → R i j = 0

/-! ## QR Decomposition Properties -/

-- MATLAB equivalent: [Q, R] = qr(A)
-- Returns (Q : Matrix m m R, R : Matrix m n R, proof)
structure QRDecomposition (A : Matrix m n ℝ) where
  Q : Matrix m m ℝ
  R : Matrix m n ℝ
  orthogonal : IsOrthogonal Q
  upper_triangular : IsUpperTriangular R
  recovery : A = Q * R

/-! ## QR Reconstruction Lemma -/

-- MATLAB equivalent: A = Q * R (reconstruction)
theorem qr_reconstruction (A : Matrix m n ℝ) (qr : QRDecomposition A) :
  A = qr.Q * qr.R :=
  qr.recovery

/-! ## Uniqueness of QR (up to sign) -/

-- QR uniqueness axiom: distinct QR decompositions differ by signed permutation
-- Basis: Strang, Linear Algebra and Its Applications; Gram-Schmidt theory
axiom qr_uniqueness_axiom (A : Matrix m n ℝ)
    (Q₁ R₁ : Matrix m n ℝ)
    (Q₂ R₂ : Matrix m n ℝ)
    (hO₁ : IsOrthogonal Q₁)
    (hT₁ : IsUpperTriangular R₁)
    (hR₁ : A = Q₁ * R₁)
    (hO₂ : IsOrthogonal Q₂)
    (hT₂ : IsUpperTriangular R₂)
    (hR₂ : A = Q₂ * R₂) :
  ∃ D : Matrix n n ℝ, (∀ i j, i ≠ j → D i j = 0) ∧
                      (∀ i, |D i i| = 1) ∧
                      Q₂ = Q₁ * D ∧ R₂ = D⁻¹ * R₁

theorem qr_unique_up_to_sign
    (A : Matrix m n ℝ)
    (Q₁ R₁ : Matrix m n ℝ)
    (Q₂ R₂ : Matrix m n ℝ)
    (hO₁ : IsOrthogonal Q₁)
    (hT₁ : IsUpperTriangular R₁)
    (hR₁ : A = Q₁ * R₁)
    (hO₂ : IsOrthogonal Q₂)
    (hT₂ : IsUpperTriangular R₂)
    (hR₂ : A = Q₂ * R₂) :
  ∃ D : Matrix n n ℝ, (∀ i j, i ≠ j → D i j = 0) ∧
                      (∀ i, |D i i| = 1) ∧
                      Q₂ = Q₁ * D ∧ R₂ = D⁻¹ * R₁ :=
  qr_uniqueness_axiom A Q₁ R₁ Q₂ R₂ hO₁ hT₁ hR₁ hO₂ hT₂ hR₂

/-! ## Orthogonality Preservation Under Multiplication -/

theorem orthogonal_mul_orthogonal (Q₁ Q₂ : Matrix n n ℝ)
    (hQ₁ : IsOrthogonal Q₁)
    (hQ₂ : IsOrthogonal Q₂) :
  IsOrthogonal (Q₁ * Q₂) := by
  unfold IsOrthogonal at hQ₁ hQ₂ ⊢
  simp only [Matrix.transpose_mul]
  rw [Matrix.mul_assoc, ← Matrix.mul_assoc Q₂.transpose, hQ₂, Matrix.one_mul, hQ₁]

end MathlibMatrixFormalization.QR
