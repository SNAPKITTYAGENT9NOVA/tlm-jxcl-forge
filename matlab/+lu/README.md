# LU Decomposition Certification Module

Formal certification of LU decomposition properties with correspondence to Lean 4 theorems.

## Overview

The LU decomposition is a fundamental matrix factorization:
```
P*A = L*U
```
where:
- **P**: Permutation matrix (row interchanges)
- **L**: Lower triangular matrix with unit diagonal
- **U**: Upper triangular matrix

This module certifies that computed LU decompositions satisfy formal invariants corresponding to theorems in the Lean 4 formalization.

## API

### `certificate = lu.certifyDecomposition(A, tolerance)`

Computes LU decomposition and verifies four formal invariants.

**Inputs:**
- `A` (m × n numeric): Input matrix
- `tolerance` (positive real, default: 1e-10): Tolerance for numerical checks

**Returns:**
- `certificate` (struct): Certification result with fields:
  - `A`: Input matrix
  - `L`: Lower triangular factor (unit diagonal)
  - `U`: Upper triangular factor
  - `P`: Permutation matrix
  - `reconstruction`: Error ||P*A - L*U|| / ||A||
  - `lowerTriangular`: Max off-diagonal in L
  - `upperTriangular`: Max below-diagonal in U
  - `unitDiagonal`: Max deviation of L diagonal from 1
  - `tolerance`: Tolerance parameter used
  - `certified`: Boolean flag (all invariants satisfied)

## Invariants

| # | Invariant | Error Metric | Formula |
|---|-----------|--------------|---------|
| 1 | Reconstruction | ||P*A - L*U|| / ||A|| | Verifies factorization |
| 2 | L lower triangular | max\|L(i,j)\| for i < j | L has zeros above diagonal |
| 3 | U upper triangular | max\|U(i,j)\| for i > j | U has zeros below diagonal |
| 4 | L unit diagonal | max\|L(i,i) - 1\| | All diagonal entries equal 1 |

## Lean 4 Correspondence

Certified properties correspond to formal theorems in `MathlibMatrixFormalization.LU`:

| MATLAB Check | Lean 4 Theorem | Formalization |
|--------------|----------------|---------------|
| P*A = L*U | `lu_recovery` | `A = P⁻¹ * L * U` |
| L lower triangular | `IsLowerTriangular` | `∀ i j, i < j → L i j = 0` |
| U upper triangular | `IsUpperTriangular` | `∀ i j, i > j → U i j = 0` |
| L unit diagonal | `IsUnitDiagonal` | `∀ i, L i i = 1` |
| Determinant | `lu_det_property` | `det(A) = det(L)*det(U)*det(P)` |

## Test Coverage

The `testLUCertification.m` suite includes 21 tests:

### Matrix Categories (9 tests)
- **Full-rank**: Square, tall, wide matrices
- **Special**: Identity, triangular, diagonal matrices
- **Numerical**: Ill-conditioned, small, large values

### Invariant Verification (4 tests)
- Reconstruction error within tolerance
- L strictly lower triangular
- U strictly upper triangular
- L has unit diagonal

### Cross-Validation (4 tests)
- Lean theorem: LU reconstruction
- Lean theorem: Lower triangularity
- Lean theorem: Unit diagonal

### Robustness (4 tests)
- Determinant property verification
- Tolerance sensitivity (strict/relaxed)
- Complex-valued matrices

## Implementation Notes

1. **Permutation Representation**: MATLAB's `lu()` returns P such that P*A = L*U (row interchanges are pre-multiplied)

2. **Unit Diagonal**: L has 1's on the diagonal by convention; the multipliers are stored in the strictly lower part

3. **Numerical Stability**: All error metrics normalize by matrix norms to account for scale invariance

4. **Tolerance Parameter**: Default 1e-10 accommodates IEEE 754 double precision (~2.22e-16 unit roundoff)

## Example Usage

```matlab
% Create and decompose a matrix
A = [4 3 2; 6 3 1; 3 2 4];

% Certify decomposition
cert = lu.certifyDecomposition(A, 1e-10);

% Check results
if cert.certified
    disp('LU decomposition certified');
    disp(sprintf('Reconstruction error: %e', cert.reconstruction));
else
    disp('Certification failed');
    disp(sprintf('Reconstruction error: %e', cert.reconstruction));
    disp(sprintf('L lower triangular error: %e', cert.lowerTriangular));
    disp(sprintf('U upper triangular error: %e', cert.upperTriangular));
    disp(sprintf('Unit diagonal error: %e', cert.unitDiagonal));
end
```

## Design Notes

### Correspondence with QR Module

The LU certification follows the same three-invariant pattern as QR:
1. Reconstruction error (factorization property)
2. Structural constraints (L lower, U upper, L unit diagonal)
3. Derived properties (determinant, conditioning)

### Extensibility Points

1. **Pivoting Strategy**: Current implementation uses MATLAB's default partial pivoting
2. **Scaling**: Could add row/column scaling analysis for ill-conditioned matrices
3. **Sparse Matrices**: Could extend to sparse LU (e.g., via `lu()` on sparse inputs)
4. **Iterative Refinement**: Could add certification of refined solutions

## References

- Golub & Van Loan (2013). *Matrix Computations* (4th ed.). Johns Hopkins University Press.
- Strang (2009). *Introduction to Linear Algebra* (4th ed.). Wellesley-Cambridge Press.
- Wilkinson (1961). Error analysis of direct methods of matrix inversion. *Journal of the ACM*, 8(3), 281-330.
