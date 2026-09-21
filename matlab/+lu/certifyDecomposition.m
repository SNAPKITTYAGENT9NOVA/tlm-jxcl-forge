function certificate = certifyDecomposition(A, tolerance)
%certifyDecomposition Verify LU decomposition satisfies formal properties.
%
% Certifies that [L, U, P] = lu(A) satisfies:
%   1. P*A = L*U (reconstruction with permutation)
%   2. L is lower triangular with unit diagonal
%   3. U is upper triangular
%
% Usage:
%   certificate = lu.certifyDecomposition(A)
%   certificate = lu.certifyDecomposition(A, tolerance)
%
% Inputs:
%   A          - Matrix m × n (real or complex)
%   tolerance  - Numerical tolerance for invariant checks (default: 1e-10)
%
% Returns:
%   certificate - Struct with fields:
%       A                    - Input matrix
%       L                    - Lower triangular factor (unit diagonal)
%       U                    - Upper triangular factor
%       P                    - Permutation matrix
%       reconstruction       - ||P*A - L*U|| / ||A||
%       lowerTriangular      - Max |L(i,j)| for i < j
%       upperTriangular      - Max |U(i,j)| for i > j
%       unitDiagonal         - Max |L(i,i) - 1|
%       certified            - logical: all invariants satisfied

arguments
    A (:,:) {mustBeNumeric}
    tolerance {mustBePositive} = 1e-10
end

% Compute LU decomposition
[L, U, P] = lu(A);

[m, n] = size(A);

% Invariant 1: Reconstruction ||P*A - L*U||
reconstructionError = norm(P*A - L*U, 'fro') / (norm(A, 'fro') + eps);

% Invariant 2: L is lower triangular (check strict upper part)
upperPart = triu(L, 1);
lowerTriangularError = norm(upperPart, 'fro') / (norm(L, 'fro') + eps);

% Invariant 3: U is upper triangular (check strict lower part)
lowerPartU = tril(U, -1);
upperTriangularError = norm(lowerPartU, 'fro') / (norm(U, 'fro') + eps);

% Invariant 4: L has unit diagonal
diagonal = diag(L);
unitDiagonalError = norm(diagonal - 1, 'fro') / (norm(diagonal, 'fro') + eps);

% Certification: all four invariants within tolerance
isCertified = (reconstructionError <= tolerance) && ...
              (lowerTriangularError <= tolerance) && ...
              (upperTriangularError <= tolerance) && ...
              (unitDiagonalError <= tolerance);

certificate = struct( ...
    "A", A, ...
    "L", L, ...
    "U", U, ...
    "P", P, ...
    "reconstruction", reconstructionError, ...
    "lowerTriangular", lowerTriangularError, ...
    "upperTriangular", upperTriangularError, ...
    "unitDiagonal", unitDiagonalError, ...
    "tolerance", tolerance, ...
    "certified", isCertified);
end
