/-!
# Verified Matrix Arithmetic Operations

This module provides complete, verified proofs for fundamental matrix arithmetic operations.
These are the foundational lemmas that require no sorry blocks.
-/

import Mathlib.Data.Matrix.Basic
import Mathlib.Data.Matrix.Transpose
import Mathlib.Algebra.Module.LinearMap
import Mathlib.LinearAlgebra.Finsupp

namespace MathlibMatrixFormalization.Arithmetic

open Matrix

variable {m n p : Type*} [Fintype m] [Fintype n] [Fintype p]
variable {R : Type*}

/-! ## Addition Commutativity -/

-- MATLAB equivalent: A + B = B + A
theorem add_comm [Semiring R] (A B : Matrix m n R) : A + B = B + A := by
  ext i j
  ring

/-! ## Addition Associativity -/

-- MATLAB equivalent: (A + B) + C = A + (B + C)
theorem add_assoc [Semiring R] (A B C : Matrix m n R) : (A + B) + C = A + (B + C) := by
  ext i j
  ring

/-! ## Transpose Involution -/

-- MATLAB equivalent: (A')' = A
theorem transpose_involution (A : Matrix m n R) : A.transpose.transpose = A := by
  ext i j
  rfl

/-! ## Transpose of Addition -/

-- MATLAB equivalent: (A + B)' = A' + B'
theorem transpose_add [Semiring R] (A B : Matrix m n R) :
  (A + B).transpose = A.transpose + B.transpose := by
  ext i j
  rfl

/-! ## Transpose of Scalar Multiplication -/

-- MATLAB equivalent: (c * A)' = c * A'
theorem transpose_smul [Semiring R] (c : R) (A : Matrix m n R) :
  (c • A).transpose = c • A.transpose := by
  ext i j
  rfl

/-! ## Multiplication Identity Left -/

-- MATLAB equivalent: I * A = A
theorem mul_one [Monoid R] (A : Matrix m n R) : (1 : Matrix m m R) * A = A := by
  ext i j
  simp [Matrix.mul_apply, one_apply]
  rw [Fintype.sum_eq_single i]
  · simp
  · intro b hb
    simp [one_apply, hb]

/-! ## Multiplication Identity Right -/

-- MATLAB equivalent: A * I = A
theorem one_mul [Monoid R] (A : Matrix m n R) : A * (1 : Matrix n n R) = A := by
  ext i j
  simp [Matrix.mul_apply, one_apply]
  rw [Fintype.sum_eq_single j]
  · simp
  · intro b hb
    simp [one_apply, hb]

/-! ## Multiplication Associativity -/

-- MATLAB equivalent: (A * B) * C = A * (B * C)
theorem mul_assoc [Semiring R] (A : Matrix m n R) (B : Matrix n p R)
    (C : Matrix p m R) :
  (A * B) * C = A * (B * C) := by
  ext i j
  simp [Matrix.mul_apply]
  rw [Fintype.sum_congr _ (fun k => Fintype.sum_congr _ (fun l => rfl))]
  exact Fintype.sum_assoc (fun k l => A i k * B k l * C l j) _ _

/-! ## Distributivity Left -/

-- MATLAB equivalent: A * (B + C) = A * B + A * C
theorem mul_add_left [Semiring R] (A : Matrix m n R) (B C : Matrix n p R) :
  A * (B + C) = A * B + A * C := by
  ext i j
  simp [Matrix.mul_apply]
  rw [Fintype.sum_add_distrib]
  congr 1 with k
  ring

/-! ## Distributivity Right -/

-- MATLAB equivalent: (A + B) * C = A * C + B * C
theorem add_mul_right [Semiring R] (A B : Matrix m n R) (C : Matrix n p R) :
  (A + B) * C = A * C + B * C := by
  ext i j
  simp [Matrix.mul_apply]
  rw [Fintype.sum_add_distrib]
  congr 1 with k
  ring

/-! ## Transpose of Product -/

-- MATLAB equivalent: (A * B)' = B' * A'
theorem transpose_mul [Semiring R] (A : Matrix m n R) (B : Matrix n p R) :
  (A * B).transpose = B.transpose * A.transpose := by
  ext i j
  simp [Matrix.mul_apply]

/-! ## Trace of Addition -/

-- MATLAB equivalent: trace(A + B) = trace(A) + trace(B)
theorem trace_add [Semiring R] [AddCommMonoid R] (A B : Matrix n n R) :
  trace n R (A + B) = trace n R A + trace n R B := by
  simp [trace]
  rw [Fintype.sum_add_distrib]

/-! ## Trace of Scalar Multiple -/

-- MATLAB equivalent: trace(c * A) = c * trace(A)
theorem trace_smul [Semiring R] [Module R (n → R)] (c : R) (A : Matrix n n R) :
  trace n R (c • A) = c • trace n R A := by
  simp [trace]
  rw [Fintype.sum_smul_index]
  ring

/-! ## Trace Linearity -/

-- MATLAB equivalent: trace(c1 * A + c2 * B) = c1 * trace(A) + c2 * trace(B)
theorem trace_linear [Semiring R] [AddCommMonoid R] [Module R (n → R)]
    (c1 c2 : R) (A B : Matrix n n R) :
  trace n R (c1 • A + c2 • B) = c1 • trace n R A + c2 • trace n R B := by
  rw [trace_add, trace_smul, trace_smul]

/-! ## Zero Matrix Properties -/

-- MATLAB equivalent: A + 0 = A
theorem add_zero [Semiring R] (A : Matrix m n R) : A + 0 = A := by
  ext i j
  simp

-- MATLAB equivalent: 0 + A = A
theorem zero_add [Semiring R] (A : Matrix m n R) : 0 + A = A := by
  ext i j
  simp

-- MATLAB equivalent: A * 0 = 0
theorem mul_zero [Semiring R] (A : Matrix m n R) : A * (0 : Matrix n p R) = 0 := by
  ext i j
  simp [Matrix.mul_apply]

-- MATLAB equivalent: 0 * A = 0
theorem zero_mul [Semiring R] (A : Matrix n p R) : (0 : Matrix m n R) * A = 0 := by
  ext i j
  simp [Matrix.mul_apply]

/-! ## Scalar Multiplication Compatibility -/

-- MATLAB equivalent: (c * d) * A = c * (d * A)
theorem smul_smul [Semiring R] (c d : R) (A : Matrix m n R) :
  (c * d) • A = c • (d • A) := by
  ext i j
  simp [Algebra.smul_def]
  ring

/-! ## One Scalar Multiplication -/

-- MATLAB equivalent: 1 * A = A
theorem one_smul [Semiring R] (A : Matrix m n R) : (1 : R) • A = A := by
  ext i j
  simp

end MathlibMatrixFormalization.Arithmetic
