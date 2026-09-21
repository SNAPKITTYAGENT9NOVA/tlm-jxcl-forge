function tests = testQRCertification
%testQRCertification Test suite for QR decomposition certification.
%
% Tests the QR certification layer:
%   1. QR reconstruction: ||A - Q*R|| ≤ tol
%   2. Orthogonality: ||Q'*Q - I|| ≤ tol
%   3. Upper triangular: R(i,j) = 0 for i > j

tests = functiontests(localfunctions);
end

%% ============================================================================
% Full-Rank Matrices
%% ============================================================================

function testQRCertifiesFullRankSquareMatrix(testCase)
% Full-rank square matrix should be certified.

A = [1 2; 3 4];
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
verifyLessThan(testCase, certificate.orthogonality, 1e-10);
verifyLessThan(testCase, certificate.upperTriangular, 1e-10);
end

function testQRCertifiesFullRankTallMatrix(testCase)
% Overdetermined system: m > n, full column rank.

A = [1 2; 3 4; 5 6];
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
verifyLessThan(testCase, certificate.orthogonality, 1e-10);
verifyLessThan(testCase, certificate.upperTriangular, 1e-10);
end

function testQRCertifiesFullRankWideMatrix(testCase)
% Underdetermined system: m < n.

A = [1 2 3; 4 5 6];
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
verifyLessThan(testCase, certificate.orthogonality, 1e-10);
verifyLessThan(testCase, certificate.upperTriangular, 1e-10);
end

%% ============================================================================
% Special Matrices
%% ============================================================================

function testQRCertifiesOrthogonalMatrix(testCase)
% QR of an already-orthogonal matrix should have Q ≈ A, R ≈ I.

theta = pi / 6;
A = [cos(theta), -sin(theta); sin(theta), cos(theta)];
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
% For orthogonal A: Q should be ±A and R should be ±I (up to sign)
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
end

function testQRCertifiesIdentityMatrix(testCase)
% QR of identity should have Q = I, R = I.

A = eye(3);
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
end

function testQRCertifiesUpperTriangularMatrix(testCase)
% QR of upper triangular R should have Q = I, R = R_in.

R_in = [1 2 3; 0 4 5; 0 0 6];
certificate = qr.certifyDecomposition(R_in, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
end

%% ============================================================================
% Numerical Stability
%% ============================================================================

function testQRCertifiesIllConditionedMatrix(testCase)
% Ill-conditioned matrix (large condition number).
% QR should still be accurate despite ill-conditioning.

A = [1e-10, 1; 1, 1];
certificate = qr.certifyDecomposition(A, 1e-8);

verifyTrue(testCase, certificate.certified, ...
    "QR should be accurate even for ill-conditioned matrices (at slightly relaxed tolerance)");
verifyLessThan(testCase, certificate.reconstruction, 1e-8);
end

function testQRCertifiesSmallMatrix(testCase)
% Very small-norm matrix.

A = 1e-15 * randn(3, 3);
certificate = qr.certifyDecomposition(A, 1e-12);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-12);
end

function testQRCertifiesLargeMatrix(testCase)
% Larger matrix (100 × 100).

A = randn(100, 100);
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
end

%% ============================================================================
% Invariant Verification
%% ============================================================================

function testQRReconstruction(testCase)
% Verify reconstruction: A = Q*R within numerical precision.

A = magic(4);
certificate = qr.certifyDecomposition(A, 1e-10);

reconstructed = certificate.Q * certificate.R;
relError = norm(A - reconstructed, 'fro') / norm(A, 'fro');

verifyLessThan(testCase, relError, 1e-10);
end

function testQROrthogonality(testCase)
% Verify orthogonality: Q' * Q ≈ I.

A = randn(5, 4);
certificate = qr.certifyDecomposition(A, 1e-10);

QtQ = certificate.Q' * certificate.Q;
I = eye(size(QtQ, 1));

relError = norm(QtQ - I, 'fro') / norm(I, 'fro');
verifyLessThan(testCase, relError, 1e-10);
end

function testQRUpperTriangular(testCase)
% Verify R is upper triangular: R(i,j) = 0 for i > j.

A = randn(6, 4);
certificate = qr.certifyDecomposition(A, 1e-10);

R = certificate.R;
[m, n] = size(R);

% Check strict lower triangular part is zero
for i = 2:m
    for j = 1:(i-1)
        verifyEqual(testCase, R(i, j), 0, 'AbsTol', 1e-10);
    end
end
end

%% ============================================================================
% Determinant Property (Lean: det(Q) = ±1)
%% ============================================================================

function testQRDeterminantOfQ(testCase)
% For square orthogonal Q, det(Q) should be ±1.

A = randn(5, 5);
certificate = qr.certifyDecomposition(A, 1e-10);

detQ = det(certificate.Q);

% det(Q) should be ±1 (for orthogonal matrix)
verifyTrue(testCase, abs(abs(detQ) - 1) < 1e-10, ...
    sprintf("det(Q) = %g should be ±1 for orthogonal matrix", detQ));
end

%% ============================================================================
% Tolerance Sensitivity
%% ============================================================================

function testQRCertificationWithTightTolerance(testCase)
% Very tight tolerance should still pass for well-conditioned matrices.

A = eye(4) + 0.01 * randn(4);
certificate = qr.certifyDecomposition(A, 1e-12);

verifyTrue(testCase, certificate.certified);
end

function testQRCertificationWithRelaxedTolerance(testCase)
% Relaxed tolerance should pass for trickier matrices.

A = [1e-10, 1; 1, 1];
certificate = qr.certifyDecomposition(A, 1e-6);

verifyTrue(testCase, certificate.certified);
end

%% ============================================================================
% Complex Matrices
%% ============================================================================

function testQRCertifiesComplexMatrix(testCase)
% QR of complex matrix.

A = [1+1i, 2+2i; 3+3i, 4+4i];
certificate = qr.certifyDecomposition(A, 1e-10);

verifyTrue(testCase, certificate.certified);
verifyLessThan(testCase, certificate.reconstruction, 1e-10);
end

%% ============================================================================
% Cross-Validation with Lean Theorems
%% ============================================================================

function testQRLeanOrthogonalDetProperty(testCase)
% Lean theorem: orthogonal_det_eq_pm_one
% For square orthogonal Q: det(Q) = 1 or det(Q) = -1

A = randn(5, 5);
certificate = qr.certifyDecomposition(A, 1e-10);

if certificate.certified
    detQ = det(certificate.Q);
    % Should satisfy det(Q) = ±1 (Lean theorem)
    verifyTrue(testCase, abs(abs(detQ) - 1) < 1e-9, ...
        "Lean: orthogonal_det_eq_pm_one");
end
end

function testQRLeanReconstruction(testCase)
% Lean theorem: qr_reconstruction
% For QRDecomposition: A = Q * R

A = randn(6, 4);
certificate = qr.certifyDecomposition(A, 1e-10);

if certificate.certified
    reconstructionError = norm(A - certificate.Q * certificate.R, 'fro') / norm(A, 'fro');
    verifyLessThan(testCase, reconstructionError, 1e-10, ...
        "Lean: qr_reconstruction");
end
end

function testQRLeanOrthogonalMulOrthogonal(testCase)
% Lean theorem: orthogonal_mul_orthogonal
% Product of two orthogonal matrices is orthogonal.

A1 = randn(4, 4);
A2 = randn(4, 4);

cert1 = qr.certifyDecomposition(A1, 1e-10);
cert2 = qr.certifyDecomposition(A2, 1e-10);

if cert1.certified && cert2.certified
    Q_product = cert1.Q * cert2.Q;
    QtQ_product = Q_product' * Q_product;
    I = eye(4);

    orthError = norm(QtQ_product - I, 'fro') / norm(I, 'fro');
    verifyLessThan(testCase, orthError, 1e-9, ...
        "Lean: orthogonal_mul_orthogonal");
end
end
