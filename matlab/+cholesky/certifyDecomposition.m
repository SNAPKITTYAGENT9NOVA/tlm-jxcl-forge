function certificate = certifyDecomposition(A, tolerance)
%certifyDecomposition Verify Cholesky decomposition satisfies formal properties.
%
% Certifies that L = chol(A, 'lower') satisfies:
%   1. A = L*L' (reconstruction)
%   2. L is lower triangular
%   3. A is symmetric positive definite (SPD)
%
% Usage:
%   certificate = cholesky.certifyDecomposition(A)
%   certificate = cholesky.certifyDecomposition(A, tolerance)
%
% Inputs:
%   A          - Symmetric positive definite matrix n × n
%   tolerance  - Numerical tolerance for invariant checks (default: 1e-10)
%
% Returns:
%   certificate - Struct with fields:
%       A                    - Input matrix
%       L                    - Lower triangular factor
%       reconstruction       - ||A - L*L'|| / ||A||
%       lowerTriangular      - Max |L(i,j)| for i < j
%       symmetry             - ||A - A'|| / ||A||
%       positiveDiagonal     - Min L(i,i)
%       certified            - logical: all invariants satisfied

arguments
    A (:,:) {mustBeNumeric}
    tolerance {mustBePositive} = 1e-10
end

% Compute Cholesky decomposition (lower triangular)
try
    L = chol(A, 'lower');
    isPositiveDefinite = true;
catch
    % If Cholesky fails, matrix is not positive definite
    isPositiveDefinite = false;
    L = zeros(size(A));  % Dummy value
end

n = size(A, 1);

% Invariant 1: Reconstruction ||A - L*L'||
if isPositiveDefinite
    reconstructionError = norm(A - L*L', 'fro') / (norm(A, 'fro') + eps);
else
    reconstructionError = inf;
end

% Invariant 2: L is lower triangular (check strict upper part)
upperPart = triu(L, 1);
lowerTriangularError = norm(upperPart, 'fro') / (norm(L, 'fro') + eps);

% Invariant 3: A is symmetric
symmetryError = norm(A - A', 'fro') / (norm(A, 'fro') + eps);

% Invariant 4: Diagonal of L is positive (positive definiteness indicator)
if isPositiveDefinite
    diagonal = diag(L);
    minDiagonal = min(diagonal);
else
    minDiagonal = -inf;
end

% Certification: all invariants within tolerance
isCertified = isPositiveDefinite && ...
              (reconstructionError <= tolerance) && ...
              (lowerTriangularError <= tolerance) && ...
              (symmetryError <= tolerance) && ...
              (minDiagonal >= -tolerance);

certificate = struct( ...
    "A", A, ...
    "L", L, ...
    "reconstruction", reconstructionError, ...
    "lowerTriangular", lowerTriangularError, ...
    "symmetry", symmetryError, ...
    "minDiagonal", minDiagonal, ...
    "isPositiveDefinite", isPositiveDefinite, ...
    "tolerance", tolerance, ...
    "certified", isCertified);
end
