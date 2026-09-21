function certificate = certifyDecomposition(A, tolerance)
%certifyDecomposition Verify QR decomposition satisfies formal properties.
%
% Certifies that [Q, R] = qr(A) satisfies:
%   1. A = Q * R (reconstruction)
%   2. Q' * Q ≈ I (orthogonality)
%   3. R is upper triangular
%
% Usage:
%   certificate = qr.certifyDecomposition(A)
%   certificate = qr.certifyDecomposition(A, tolerance)
%
% Inputs:
%   A          - Matrix m × n (real or complex)
%   tolerance  - Numerical tolerance for invariant checks (default: 1e-10)
%
% Returns:
%   certificate - Struct with fields:
%       A                    - Input matrix
%       Q                    - Computed orthogonal factor
%       R                    - Computed upper triangular factor
%       reconstruction       - ||A - Q*R|| / ||A||
%       orthogonality        - ||Q'*Q - I|| / ||I||
%       upperTriangular      - Max |R(i,j)| for i > j
%       certified            - logical: all invariants satisfied

arguments
    A (:,:) {mustBeNumeric}
    tolerance {mustBePositive} = 1e-10
end

% Compute QR decomposition
[Q, R] = qr(A);

[m, n] = size(A);

% Invariant 1: Reconstruction ||A - Q*R||
reconstructionError = norm(A - Q*R, 'fro') / norm(A, 'fro');

% Invariant 2: Orthogonality ||Q'*Q - I||
orthogonalityError = norm(Q' * Q - eye(m), 'fro') / norm(eye(m), 'fro');

% Invariant 3: Upper triangular (check strict lower part)
lowerPart = tril(R, -1);
upperTriangularError = norm(lowerPart, 'fro') / (norm(R, 'fro') + eps);

% Certification: all three invariants within tolerance
isCertified = (reconstructionError <= tolerance) && ...
              (orthogonalityError <= tolerance) && ...
              (upperTriangularError <= tolerance);

certificate = struct( ...
    "A", A, ...
    "Q", Q, ...
    "R", R, ...
    "reconstruction", reconstructionError, ...
    "orthogonality", orthogonalityError, ...
    "upperTriangular", upperTriangularError, ...
    "tolerance", tolerance, ...
    "certified", isCertified);
end
