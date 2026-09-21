# SVD (Singular Value Decomposition) Certification Module

Formal certification of SVD properties with correspondence to Lean 4 theorems.

## Overview

The Singular Value Decomposition is a fundamental matrix factorization with applications to rank estimation, matrix approximation, and pseudoinverse computation:
```
A = U*S*V'
```
where:
- **U**: Orthogonal matrix (left singular vectors)
- **S**: Diagonal matrix of singular values (σ ≥ 0)
- **V**: Orthogonal matrix (right singular vectors)

This module certifies that computed SVD satisfies formal invariants corresponding to theorems in the Lean 4 formalization.

## API

### `certificate = svd.certifyDecomposition(A, tolerance)`

Computes SVD and verifies four formal invariants.

**Inputs:**
- `A` (m × n numeric): Input matrix
- `tolerance` (positive real, default: 1e-10): Tolerance for numerical checks

**Returns:**
- `certificate` (struct): Certification result with fields:
  - `A`: Input matrix
  - `U`: Left orthogonal factor (m × m)
  - `S`: Singular values (m × n diagonal matrix)
  - `V`: Right orthogonal factor (n × n)
  - `reconstruction`: Error ||A - U*S*V'|| / ||A||
  - `orthogonalityU`: Error ||U'*U - I||
  - `orthogonalityV`: Error ||V'*V - I||
  - `diagonal`: Max off-diagonal in S
  - `minSingularValue`: Smallest singular value (should be ≥ 0)
  - `tolerance`: Tolerance parameter used
  - `certified`: Boolean flag (all invariants satisfied)

## Invariants

| # | Invariant | Error Metric | Formula |
|---|-----------|--------------|---------|
| 1 | Reconstruction | ||A - U*S*V'|| / ||A|| | Verifies factorization |
| 2 | U orthogonal | ||U'*U - I|| / ||I|| | U'*U = I (or full rank subset) |
| 3 | V orthogonal | ||V'*V - I|| / ||I|| | V'*V = I |
| 4 | S diagonal | max\|S(i,j)\| for i ≠ j | S has zeros off-diagonal |
| 5 | Non-negative | min(singular values) | All σ ≥ 0 |

## Lean 4 Correspondence

Certified properties correspond to formal theorems in `MathlibMatrixFormalization.SVD`:

| MATLAB Check | Lean 4 Theorem | Formalization |
|--------------|----------------|---------------|
| A = U*S*V' | `svd_recovery` | `A = U * S * V.transpose` |
| U orthogonal | `svd_U_orthogonal` | `U.transpose * U = I` |
| V orthogonal | `svd_V_orthogonal` | `V.transpose * V = I` |
| S diagonal | `IsdiagonalMatrix` | `∀ i j, i ≠ j → S i j = 0` |
| Non-negative | `svd_singular_values_nonnegative` | `∀ i, S i i ≥ 0` |
| Determinant | `svd_det_property` | `det(A) = ±∏σ` |

## Test Coverage

The `testSVDCertification.m` suite includes 24 tests:

### Matrix Categories (9 tests)
- **Full-rank**: Square, tall, wide matrices
- **Special**: Identity, rank-deficient, diagonal matrices
- **Numerical**: Ill-conditioned, small, large values

### Invariant Verification (5 tests)
- Reconstruction error within tolerance
- U orthogonality (U'*U = I)
- V orthogonality (V'*V = I)
- S diagonal with off-diagonal elements
- Non-negative singular values

### Derived Properties (3 tests)
- Determinant = ±product of singular values
- Rank = number of non-zero singular values
- Singular values in decreasing order

### Cross-Validation (5 tests)
- Lean theorem: SVD reconstruction
- Lean theorem: U orthogonality
- Lean theorem: V orthogonality
- Lean theorem: S diagonal property
- Lean theorem: Non-negative singular values

### Robustness (2 tests)
- Tolerance sensitivity (strict/relaxed)
- Complex-valued matrices

## Implementation Notes

1. **Singular Value Ordering**: MATLAB's `svd()` returns singular values in decreasing order

2. **Orthogonality of Rectangular U**: For tall matrices (m > n), the first n columns of U form an orthonormal basis; for wide matrices (m < n), U is square

3. **Determinant Property**: For square matrices, det(A) = ±∏σ where the sign depends on sign of det(U)*det(V)

4. **Rank Computation**: rank(A) = number of singular values σ > tolerance

5. **Numerical Stability**: All error metrics normalize by matrix norms to account for scale invariance

## Example Usage

```matlab
% Create test matrix
A = [1 2; 3 4; 5 6];  % 3x2 matrix

% Certify SVD
cert = svd.certifyDecomposition(A, 1e-10);

% Check results
if cert.certified
    disp('SVD certified');
    disp(sprintf('Reconstruction error: %e', cert.reconstruction));
    disp(sprintf('Singular values: %s', mat2str(diag(cert.S))));
else
    disp('Certification failed');
end

% Compute approximate rank
rank_A = sum(diag(cert.S) > 1e-10);
disp(sprintf('Numerical rank: %d', rank_A));
```

## Design Notes

### Correspondence with QR Module

The SVD certification follows the same pattern as QR:
1. Reconstruction error (factorization property)
2. Orthogonality constraints (U and V orthogonal)
3. Derived properties (rank, determinant, condition number)

### Extensibility Points

1. **Truncated SVD**: Could add certification for low-rank approximations
2. **Economical SVD**: Could certify economy-size decomposition
3. **Generalized SVD**: Could extend to matrix pairs (A, B)
4. **Pseudoinverse**: Could verify A⁺ = V * S⁺ * U'

## References

- Golub & Van Loan (2013). *Matrix Computations* (4th ed.). Johns Hopkins University Press. Section 2.5.
- Strang (2009). *Introduction to Linear Algebra* (4th ed.). Wellesley-Cambridge Press. Chapter 7.
- Trefethen & Bau (1997). *Numerical Linear Algebra*. SIAM. Lectures 4-6.
- Demmel (1997). *Applied Numerical Linear Algebra*. SIAM. Chapter 3.
