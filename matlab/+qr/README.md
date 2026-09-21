# QR Decomposition Certification

Formal verification of QR decomposition via executable certification contracts.

## Overview

The QR certification module applies the executable certification framework to QR decomposition:

$$\text{Certified}(A) = \text{Reconstruction}(A, Q, R) \land \text{Orthogonal}(Q) \land \text{UpperTriangular}(R)$$

Where:
- **Reconstruction:** $\|A - QR\|_F / \|A\|_F \leq \tau$
- **Orthogonality:** $\|Q^T Q - I\|_F / \|I\|_F \leq \tau$
- **Upper Triangular:** $\max_{i>j} |R_{ij}| \leq \tau$ (in practice, numerically zero)

## Core Function

### `qr.certifyDecomposition` — QR Verification

```matlab
certificate = qr.certifyDecomposition(A)
certificate = qr.certifyDecomposition(A, tolerance)
```

**Input:**
- `A` — Matrix m × n (real or complex)
- `tolerance` — Numerical tolerance (default: 1e-10)

**Output:**
- `certificate` — Struct with fields:
  - `A` — Input matrix
  - `Q` — Computed orthogonal factor
  - `R` — Computed upper triangular factor
  - `reconstruction` — Relative reconstruction error
  - `orthogonality` — Relative orthogonality error
  - `upperTriangular` — Relative upper triangular error
  - `tolerance` — Tolerance used
  - `certified` — logical: all invariants within tolerance

**Example:**

```matlab
A = randn(10, 8);
cert = qr.certifyDecomposition(A, 1e-10);

if cert.certified
    fprintf('QR decomposition certified.\n');
    fprintf('  Reconstruction error: %.2e\n', cert.reconstruction);
    fprintf('  Orthogonality error: %.2e\n', cert.orthogonality);
else
    fprintf('QR decomposition failed certification.\n');
end
```

## Test Suite

Run the comprehensive test suite:

```matlab
cd matlab
results = runtests('tests/testQRCertification.m');
disp(table(results))
```

### Test Coverage

| Test | Purpose | Count |
|------|---------|-------|
| Full-rank matrices | Square, tall, wide | 3 |
| Special matrices | Orthogonal, identity, triangular | 3 |
| Numerical stability | Ill-conditioned, small, large | 3 |
| Invariant verification | Reconstruction, orthogonality, upper triangular | 3 |
| Determinant property | Lean: det(Q) = ±1 | 1 |
| Tolerance sensitivity | Tight and relaxed tolerances | 2 |
| Complex matrices | Complex-valued input | 1 |
| Cross-validation with Lean | 3 Lean theorems verified | 3 |
| **Total** | | **19 tests** |

## Correspondence with Lean 4

### Lean Theorems

The MATLAB certification verifies these Lean 4 theorems on finite instances:

**Proved Theorems:**

```lean
theorem orthogonal_det_eq_pm_one (Q : Matrix n n R) :
  IsOrthogonal Q → det Q = 1 ∨ det Q = -1

theorem qr_reconstruction (A : Matrix m n ℝ) (qr : QRDecomposition A) :
  A = qr.Q * qr.R

theorem orthogonal_mul_orthogonal (Q₁ Q₂ : Matrix n n ℝ) :
  IsOrthogonal Q₁ → IsOrthogonal Q₂ → IsOrthogonal (Q₁ * Q₂)
```

**Deferred Theorems:**

```lean
theorem orthogonal_inv_eq_transpose (Q : Matrix n n R) :
  IsOrthogonal Q → Q⁻¹ = Q.transpose
  -- Requires LinearEquiv machinery

theorem qr_unique_up_to_sign (A : Matrix m n ℝ) (Q₁ R₁ Q₂ R₂) :
  A = Q₁ * R₁ → A = Q₂ * R₂ → ... → (∃ D, Q₂ = Q₁ * D ∧ R₂ = D⁻¹ * R₁)
  -- Non-trivial uniqueness proof
```

### MATLAB-Lean Integration

1. **Executable verification:** MATLAB tests verify finite instances
2. **Universal correctness:** Lean theorems prove universal properties
3. **Numerical bridge:** MATLAB uses IEEE 754 arithmetic; Lean proves over abstract fields
4. **Regression coverage:** Bounded exhaustive tests on finite domains

## Design Notes

1. **Tolerance-based certification** — Numerical errors are inevitable; use floating-point tolerance
2. **Three invariants** — Reconstruction, orthogonality, and upper triangular structure
3. **Lean correspondence** — Each MATLAB test maps to a Lean theorem
4. **No implicit claims** — Finite MATLAB tests do not prove ∀A, QR is certified

## Next Extensions

- **LU factorization** — Similar structure (lower triangular + upper triangular)
- **SVD decomposition** — Orthogonality of U and V, diagonal structure of Σ
- **Cholesky factorization** — Lower triangular + symmetric positive definite
- **Eigenvalue decomposition** — Orthogonality and spectral properties

---

**Co-Authored-By:** Claude Haiku 4.5 <noreply@anthropic.com>
