function certificate = certifyDecomposition(A, tolerance)
%certifyDecomposition Verify SVD satisfies formal properties.
%
% Certifies that [U, S, V] = svd(A) satisfies:
%   1. A = U*S*V' (reconstruction)
%   2. U is orthogonal (U'*U = I)
%   3. V is orthogonal (V'*V = I)
%   4. S is diagonal with nonnegative singular values
%
% Usage:
%   certificate = svd.certifyDecomposition(A)
%   certificate = svd.certifyDecomposition(A, tolerance)
%
% Inputs:
%   A          - Matrix m × n (real or complex)
%   tolerance  - Numerical tolerance for invariant checks (default: 1e-10)
%
% Returns:
%   certificate - Struct with fields:
%       A                    - Input matrix
%       U                    - Left orthogonal factor
%       S                    - Singular values (diagonal matrix)
%       V                    - Right orthogonal factor
%       reconstruction       - ||A - U*S*V'|| / ||A||
%       orthogonalityU       - ||U'*U - I|| / ||I||
%       orthogonalityV       - ||V'*V - I|| / ||I||
%       diagonal             - Max |S(i,j)| for i ≠ j
%       nonnegative          - Min singular value
%       certified            - logical: all invariants satisfied

arguments
    A (:,:) {mustBeNumeric}
    tolerance {mustBePositive} = 1e-10
end

% Compute SVD
[U, S, V] = svd(A);


% Invariant 1: Reconstruction ||A - U*S*V'||
reconstructionError = norm(A - U*S*V', 'fro') / (norm(A, 'fro') + eps);

% Invariant 2: U is orthogonal (U is m x m from the full SVD)
UtU = U' * U;
I_U = eye(size(U, 2));
orthogonalityU = norm(UtU - I_U, 'fro') / norm(I_U, 'fro');

% Invariant 3: V is orthogonal (V is n x n)
VtV = V' * V;
I_V = eye(size(V, 2));
orthogonalityV = norm(VtV - I_V, 'fro') / norm(I_V, 'fro');

% Invariant 4: S is diagonal with nonnegative singular values
% (S is m x n, so mask the diagonal rather than rebuilding a square diag)
offDiagonal = S .* ~eye(size(S));
diagonalError = norm(offDiagonal, 'fro') / (norm(S, 'fro') + eps);

singularValues = diag(S);
minSingularValue = min(singularValues);

% Certification: all invariants within tolerance
isCertified = (reconstructionError <= tolerance) && ...
              (orthogonalityU <= tolerance) && ...
              (orthogonalityV <= tolerance) && ...
              (diagonalError <= tolerance) && ...
              (minSingularValue >= -tolerance);

certificate = struct( ...
    "A", A, ...
    "U", U, ...
    "S", S, ...
    "V", V, ...
    "reconstruction", reconstructionError, ...
    "orthogonalityU", orthogonalityU, ...
    "orthogonalityV", orthogonalityV, ...
    "diagonal", diagonalError, ...
    "minSingularValue", minSingularValue, ...
    "tolerance", tolerance, ...
    "certified", isCertified);
end
