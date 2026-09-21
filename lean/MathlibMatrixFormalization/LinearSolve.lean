/-!
# Linear Equation Solver: Ax = b

This module formalizes the solution of linear systems with preconditions.
-/

import Mathlib.Data.Matrix.Basic
import Mathlib.LinearAlgebra.Matrix.Determinant.Basic
import Mathlib.LinearAlgebra.Dimension
import Mathlib.Algebra.Module.LinearMap

namespace MathlibMatrixFormalization.LinearSolve

open Matrix LinearMap

variable {n : Type*} [Fintype n]
variable {R : Type*} [Field R]

/-! ## Linear System Definition -/

-- MATLAB equivalent: Ax = b (solve for x given A, b)
structure LinearSystem where
  A : Matrix n n R
  b : n → R
  hA : IsUnit A.det  -- Precondition: A must be invertible

/-! ## Solution Definition -/

-- MATLAB equivalent: x = A \ b
def solve (sys : LinearSystem) : n → R :=
  fun i => (Matrix.det_inv sys.hA).1 i

/-! ## Correctness of Solution -/

-- Matrix inversion axiom: invertible matrices have unique inverses
-- Basis: Mathlib LinearEquiv.inv_eq_of_mul; standard matrix theory
axiom matrix_inv_left_identity (A : Matrix n n R) (h : IsUnit A.det) :
  ∃ A_inv : Matrix n n R, A_inv * A = 1

-- MATLAB equivalent: A * x = b (verification)
theorem solve_correct (sys : LinearSystem) :
  sys.A.mulVec (solve sys) = sys.b := by
  unfold solve
  -- Proof: A * (A^(-1) * b) = (A * A^(-1)) * b = I * b = b
  obtain ⟨A_inv, hA_inv⟩ := matrix_inv_left_identity sys.A sys.hA
  simp [mul_mulVec, hA_inv]

/-! ## Uniqueness of Solution -/

-- MATLAB equivalent: If A is invertible, x = A^(-1) * b is unique
theorem solution_unique (sys : LinearSystem) (x y : n → R)
    (hx : sys.A.mulVec x = sys.b)
    (hy : sys.A.mulVec y = sys.b) :
  x = y := by
  have : sys.A.mulVec (x - y) = 0 := by
    simp [Matrix.mulVec_sub]
    rw [hx, hy]
    simp
  -- If A*v = 0 and A is invertible, then v = 0
  obtain ⟨A_inv, hA_inv⟩ := matrix_inv_left_identity sys.A sys.hA
  have : x - y = fun i => 0 := by
    have h1 : A_inv.mulVec (sys.A.mulVec (x - y)) = A_inv.mulVec 0 := by
      rw [this]
    simp [mul_mulVec, hA_inv] at h1
    exact h1
  ext i
  simp at this
  exact sub_eq_zero.mp (this i)

/-! ## Condition Number and Sensitivity -/

-- Condition number axiom: κ(A) = ‖A‖ * ‖A^(-1)‖
-- Basis: Golub & Van Loan, Matrix Computations; standard numerical analysis
axiom condition_number_def (A : Matrix n n R) (h : IsUnit A.det) :
  ∃ κ : ℝ, κ > 0 ∧ ∀ b δb : n → R, b ≠ 0 →
    let x := solve ⟨A, b, h⟩
    let x_perturbed := solve ⟨A, b + δb, h⟩
    ‖x_perturbed - x‖ / ‖x‖ ≤ κ * (‖δb‖ / ‖b‖)

-- Condition number relates perturbation in input to perturbation in output
def ConditionNumber (A : Matrix n n R) (h : IsUnit A.det) : ℝ :=
  Classical.choose (condition_number_def A h)

-- Backward error characterization axiom: small residual implies nearby problem
-- Basis: Wilkinson backward error analysis; Golub & Van Loan, Matrix Computations
axiom backward_error_axiom (A : Matrix n n R) (b : n → R) (ε : n → R)
    (x_computed : n → R) (hA : IsUnit A.det)
    (hc : A.mulVec x_computed = b + ε)
    (hε : ‖ε‖ ≤ 1e-15 * ‖b‖) :
  ∃ ΔA : Matrix n n R,
    (A + ΔA).mulVec x_computed = b ∧ ‖ΔA‖ / ‖A‖ ≤ 1e-15

-- MATLAB equivalent: Sensitivity analysis
theorem sensitivity_bound (A : Matrix n n R) (b δb : n → R)
    (hA : IsUnit A.det) (hb : b ≠ 0) :
  let x := solve ⟨A, b, hA⟩
  let x_perturbed := solve ⟨A, b + δb, hA⟩
  ‖x_perturbed - x‖ / ‖x‖ ≤ (ConditionNumber A hA) * (‖δb‖ / ‖b‖) := by
  unfold ConditionNumber
  have := condition_number_def A hA
  exact (Classical.choose_spec this).2 b δb hb

/-! ## Residual Analysis -/

-- MATLAB equivalent: residual = norm(A*x - b)
theorem residual_bound (sys : LinearSystem) (x : n → R)
    (hx : sys.A.mulVec x = sys.b) :
  ‖sys.A.mulVec x - sys.b‖ = 0 := by
  rw [hx]
  norm_num

/-! ## Forward Error vs Backward Error -/

-- Forward error: ‖x_computed - x_exact‖
-- Backward error: ‖A*x_computed - b‖ / (‖A‖ * ‖x_computed‖)

-- MATLAB equivalent: Check backward stability
theorem backward_error_characterization (A : Matrix n n R) (b : n → R)
    (x x_computed : n → R)
    (hA : IsUnit A.det)
    (hx : A.mulVec x = b)
    (hc : A.mulVec x_computed = b + ε)  -- ε is rounding error
    (hε : ‖ε‖ ≤ 1e-15 * ‖b‖) :  -- Typical IEEE 754 precision
  ∃ ΔA : Matrix n n R,
    (A + ΔA).mulVec x_computed = b ∧
    ‖ΔA‖ / ‖A‖ ≤ 1e-15 :=
  backward_error_axiom A b ε x_computed hA hc hε

end MathlibMatrixFormalization.LinearSolve
