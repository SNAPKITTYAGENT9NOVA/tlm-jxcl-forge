# Cholesky Decomposition Certification Module

Formal certification of Cholesky decomposition properties with correspondence to Lean 4 theorems.

## Overview

The Cholesky decomposition is the most efficient method for solving linear systems with symmetric positive definite matrices:
```
A = L*L'
```
where:
- **A**: Symmetric positive definite matrix (n × n)
- **L**: Lower triangular matrix with positive diagonal

This module certifies that computed Cholesky decompositions satisfy formal invariants corresponding to theorems in the Lean 4 formalization.

## API

### `certificate = cholesky.certifyDecomposition(A, tolerance)`

Computes Cholesky decomposition and verifies three formal invariants.

**Inputs:**
- `A` (n × n symmetric numeric): Input matrix
- `tolerance` (positive real, default: 1e-10): Tolerance for numerical checks

**Returns:**
- `certificate` (struct): Certification result with fields:
  - `A`: Input matrix
  - `L`: Lower triangular factor
  - `reconstruction`: Error ||A - L*L'|| / ||A||
  - `lowerTriangular`: Max |L(i,j)| for i < j
  - `symmetry`: Error ||A - A'|| / ||A||
  - `minDiagonal`: Minimum diagonal value min(L(i,i))
  - `isPositiveDefinite`: Boolean flag (Cholesky succeeded)
  - `tolerance`: Tolerance parameter used
  - `certified`: Boolean flag (all invariants satisfied)

## Invariants

| # | Invariant | Error Metric | Formula |
|---|-----------|--------------|---------|
| 1 | Reconstruction | ||A - L*L'|| / ||A|| | Verifies factorization |
| 2 | L lower triangular | max\|L(i,j)\| for i < j | L has zeros above diagonal |
| 3 | A symmetric | ||A - A'|| / ||A|| | A is symmetric |
| 4 | L positive diagonal | min(L(i,i)) | Diagonal entries > 0 |
| 5 | Positive definite | Cholesky success | A is SPD (necessary condition) |

## Lean 4 Correspondence

Certified properties correspond to formal theorems in `MathlibMatrixFormalization.Cholesky`:

| MATLAB Check | Lean 4 Theorem | Formalization |
|--------------|----------------|---------------|
| A = L*L' | `cholesky_recovery` | `A = L * L.transpose` |
| L lower triangular | `IsLowerTriangular` | `∀ i j, i < j → L i j = 0` |
| L positive diagonal | `IsPositiveDiagonal` | `∀ i, L i i > 0` |
| A symmetric | `IsSymmetric` | `A = A.transpose` |
| SPD property | `IsPositiveDefinite` | All eigenvalues > 0 |
| Uniqueness | `cholesky_unique` | Unique L with positive diagonal |

## Test Coverage

The `testCholeskyCertification.m` suite includes 28 tests:

### SPD Matrices (3 tests)
- Well-formed symmetric positive definite matrices
- Detection of symmetry violations
- Positive definiteness verification

### Special Matrices (5 tests)
- Identity matrix (L = I)
- Scaled identity (L = √diag)
- Small diagonal matrices
- Non-SPD (indefinite, negative, zero)

### Numerical Stability (2 tests)
- Ill-conditioned SPD matrices
- Large-scale matrices (n=100)

### Invariant Verification (4 tests)
- Reconstruction error ||A - L*L'||
- Lower triangularity of L
- Positive diagonal entries
- Symmetry of A

### Derived Properties (2 tests)
- Determinant property: det(A) = (det(L))²
- Condition number behavior

### Cross-Validation (5 tests)
- Lean theorem: Cholesky reconstruction
- Lean theorem: Lower triangularity
- Lean theorem: Positive diagonal
- Lean theorem: Symmetry
- Complex Hermitian matrices

### Robustness (2 tests)
- Tolerance sensitivity (strict/relaxed)
- Complex Hermitian matrices

## Implementation Notes

1. **SPD Detection**: Cholesky via `chol()` fails gracefully if A is not positive definite; the `isPositiveDefinite` flag indicates success

2. **Symmetry Check**: The module verifies A is symmetric before and after decomposition; asymmetric input automatically fails certification

3. **Positive Diagonal**: The Cholesky factorization guarantees positive diagonal if the computation succeeds; the check is redundant but explicitly verifies the property

4. **Efficiency**: Cholesky is O(n³/3) flops compared to O(n³) for LU; about 2x faster for SPD systems

5. **Determinant Property**: det(A) = (det(L))² provides an alternative way to check determinant while avoiding determinant underflow/overflow

## Example Usage

```matlab
% Create a symmetric positive definite matrix
A = [4 2 1; 2 5 2; 1 2 3];

% Certify Cholesky decomposition
cert = cholesky.certifyDecomposition(A, 1e-10);

% Check results
if cert.isPositiveDefinite
    if cert.certified
        disp('Cholesky decomposition certified');
        disp(sprintf('Reconstruction error: %e', cert.reconstruction));
        
        % Solve system Ax = b using Cholesky
        b = [1; 2; 3];
        y = cert.L \ b;              % Forward substitution
        x = cert.L' \ y;             % Back substitution
        disp(sprintf('Solution: %s', mat2str(x)));
    else
        disp('Cholesky computed but certification failed');
    end
else
    disp('Matrix is not positive definite');
end
```

## Design Notes

### Correspondence with QR/LU Modules

The Cholesky certification follows the same pattern as QR and LU:
1. Decomposition computation (with failure detection)
2. Invariant verification (three structural properties)
3. Derived properties (determinant, condition number)

### Efficiency vs. Correctness Trade-off

**Efficiency**: Only computes Cholesky once; verifies via reconstruction error

**Correctness**: Directly checks each invariant; catches numerical errors early

The certification is designed to catch failures that might occur due to:
- Rounding errors during factorization
- Compiler/hardware differences
- Numerical instability in ill-conditioned systems

### Extensibility Points

1. **Incomplete Cholesky**: Could certify incomplete factorization for preconditioning
2. **Block Cholesky**: Could extend to block-wise decomposition
3. **Iterative Refinement**: Could verify refined solutions
4. **Schur Complement**: Could certify bordered Cholesky for sensitivity analysis

## References

- Golub & Van Loan (2013). *Matrix Computations* (4th ed.). Johns Hopkins University Press. Section 4.2.
- Strang (2009). *Introduction to Linear Algebra* (4th ed.). Wellesley-Cambridge Press. Chapter 4.
- Higham (2002). *Accuracy and Stability of Numerical Algorithms* (2nd ed.). SIAM. Chapters 10-11.
- Dennis & Schnabel (1983). *Numerical Methods for Unconstrained Optimization and Nonlinear Equations*. Prentice Hall. Appendix A.

## Security Notes

Cholesky decomposition is widely used in cryptographic protocols and numerical solvers. This certification module helps ensure:
1. Correct implementation of the algorithm
2. Absence of silent failures on non-SPD matrices
3. Numerical stability for well-conditioned systems
4. Deterministic behavior across platforms
