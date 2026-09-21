/-!
# Basic Matrix Properties and Type Definitions

This module establishes fundamental properties of matrices using Mathlib's `Matrix` type.
-/

import Mathlib.Data.Matrix.Basic
import Mathlib.Data.Matrix.Transpose
import Mathlib.LinearAlgebra.Matrix.Adjugate.Basic
import Mathlib.Data.Matrix.Invertible
import Mathlib.LinearAlgebra.Determinant.Basic

namespace MathlibMatrixFormalization.Basic

open Matrix

variable {m n p : Type*} [Fintype m] [Fintype n] [Fintype p]
variable {R : Type*} [Semiring R]

/-! ## Dimension Properties -/

-- MATLAB equivalent: size(A)
theorem matrix_shape (A : Matrix m n R) :
  (Fintype.card m, Fintype.card n) = (Fintype.card m, Fintype.card n) :=
  rfl

/-! ## Matrix Transpose Properties -/

-- MATLAB equivalent: A'
theorem transpose_of_transpose (A : Matrix m n R) :
  A.transpose.transpose = A := by
  ext i j
  rfl

theorem transpose_add [Add R] (A B : Matrix m n R) :
  (A + B).transpose = A.transpose + B.transpose := by
  ext i j
  rfl

/-! ## Matrix Multiplication Associativity -/

-- MATLAB equivalent: (A * B) * C = A * (B * C)
theorem mul_assoc [Semiring R] (A : Matrix m n R) (B : Matrix n p R)
    (C : Matrix p m R) :
  (A * B) * C = A * (B * C) := by
  ext i j
  simp [Matrix.mul_apply]
  rw [Fintype.sum_congr]
  · exact Fintype.sum_assoc (fun i k l => A i k * B k l * C l j) _ _
  · intro k
    rfl

/-! ## Identity Matrix Properties -/

-- MATLAB equivalent: I = eye(n)
theorem mul_one_eq_self [Monoid R] (A : Matrix n n R) :
  A * 1 = A := by
  ext i j
  simp [Matrix.mul_apply, one_apply]

theorem one_mul_eq_self [Monoid R] (A : Matrix n n R) :
  1 * A = A := by
  ext i j
  simp [Matrix.mul_apply, one_apply]

/-! ## Trace Properties -/

-- MATLAB equivalent: trace(A)
theorem trace_add [Semiring R] [AddCommMonoid R] (A B : Matrix n n R) :
  Matrix.trace n R (A + B) = Matrix.trace n R A + Matrix.trace n R B := by
  simp [Matrix.trace]
  rw [Fintype.sum_add_distrib]

theorem trace_transpose [Semiring R] [AddCommMonoid R] (A : Matrix n n R) :
  Matrix.trace n R A.transpose = Matrix.trace n R A := by
  simp [Matrix.trace]

/-! ## Determinant Properties for Square Matrices -/

variable [CommRing R]

-- MATLAB equivalent: det(A * B) = det(A) * det(B)
theorem det_mul (A B : Matrix n n R) :
  Matrix.det (A * B) = Matrix.det A * Matrix.det B :=
  Matrix.det_mul A B

-- MATLAB equivalent: det(I) = 1
theorem det_one :
  Matrix.det (1 : Matrix n n R) = 1 :=
  Matrix.det_one

-- MATLAB equivalent: det(A') = det(A)
theorem det_transpose (A : Matrix n n R) :
  Matrix.det A.transpose = Matrix.det A :=
  Matrix.det_transpose A

end MathlibMatrixFormalization.Basic
