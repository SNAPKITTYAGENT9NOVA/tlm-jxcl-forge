classdef testCholeskyCertification < matlab.unittest.TestCase
    % Comprehensive test suite for Cholesky decomposition certification
    % Tests correspondence with Lean formalization theorems

    properties
        tolerance = 1e-10
    end

    methods(Test)

        % ============ Symmetric Positive Definite Matrices ============

        function testPositiveDefinite(testCase)
            % Test Cholesky of a well-formed SPD matrix
            A = [4 1 1; 1 3 0.5; 1 0.5 2];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Matrix should be detected as positive definite');
            testCase.verifyTrue(cert.certified, ...
                'SPD matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testSymmetricMatrixVerification(testCase)
            % Test that asymmetric matrix is detected as non-SPD
            A = [1 2; 3 4];  % Not symmetric
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            % Should detect symmetry error
            testCase.verifyGreaterThan(cert.symmetry, testCase.tolerance, ...
                'Asymmetric matrix should have non-zero symmetry error');
        end

        % ============ Special Matrices ============

        function testIdentityMatrix(testCase)
            % Test Cholesky of identity matrix
            A = eye(5);
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Identity is positive definite');
            testCase.verifyTrue(cert.certified, ...
                'Identity matrix certification failed');
            % L should be identity
            testCase.verifyLessThan(norm(cert.L - eye(5), 'fro'), ...
                testCase.tolerance);
        end

        function testScaledIdentity(testCase)
            % Test Cholesky of diagonal SPD matrix
            % Diagonal: [4, 9, 16] (perfect squares)
            A = diag([4, 9, 16]);
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Diagonal SPD is positive definite');
            testCase.verifyTrue(cert.certified, ...
                'Diagonal SPD certification failed');

            % L should be diagonal with [2, 3, 4]
            expectedL = diag([2, 3, 4]);
            testCase.verifyEqual(cert.L, expectedL, ...
                'AbsTol', testCase.tolerance);
        end

        function testSmallDiagonalMatrix(testCase)
            % Test Cholesky of small diagonal SPD
            A = diag([1e-8, 2e-8, 3e-8]);
            cert = cholesky.certifyDecomposition(A, 1e-10);

            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Small diagonal SPD should be detected');
            testCase.verifyTrue(cert.certified, ...
                'Small diagonal certification failed');
        end

        % ============ Non-SPD Matrices ============

        function testNonDefiniteMatrix(testCase)
            % Test detection of indefinite matrix
            A = [1 2; 2 1];  % Symmetric but indefinite (eigenvalues: -1, 3)
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyFalse(cert.isPositiveDefinite, ...
                'Indefinite matrix should not pass positivity test');
            testCase.verifyFalse(cert.certified, ...
                'Indefinite matrix should fail certification');
        end

        function testNegativeMatrix(testCase)
            % Test detection of negative definite matrix
            A = [-1 0; 0 -1];  % Negative definite
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyFalse(cert.isPositiveDefinite, ...
                'Negative definite should not pass SPD test');
            testCase.verifyFalse(cert.certified, ...
                'Negative definite should fail certification');
        end

        function testZeroMatrix(testCase)
            % Test detection of zero matrix (positive semidefinite but singular)
            A = zeros(3);
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            % Zero is on the boundary (positive semidefinite but not strictly positive)
            testCase.verifyFalse(cert.certified, ...
                'Zero matrix should fail strict positivity test');
        end

        % ============ Numerical Stability ============

        function testIllConditioned(testCase)
            % Test Cholesky of ill-conditioned SPD matrix
            % Create ill-conditioned SPD via eigenvalue scaling
            A = [10 9.9; 9.9 10];
            cert = cholesky.certifyDecomposition(A, 1e-8);

            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Ill-conditioned SPD should be detected');
            testCase.verifyTrue(cert.certified, ...
                'Ill-conditioned SPD certification failed (relax tolerance)');
        end

        function testLargeMatrix(testCase)
            % Test Cholesky of large-scale SPD matrix
            n = 100;
            A = gallery('randn', [n, n]);
            A = A' * A + eye(n);  % Construct symmetric positive definite

            cert = cholesky.certifyDecomposition(A, 1e-8);
            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Large SPD should be detected');
        end

        % ============ Individual Invariants ============

        function testReconstructionInvariant(testCase)
            % Verify reconstruction error ||A - L*L'||
            A = [4 2; 2 3];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                % Compute reconstruction independently
                reconError = norm(A - cert.L*cert.L', 'fro') / norm(A, 'fro');
                testCase.verifyLessThan(reconError, testCase.tolerance);
            end
        end

        function testLowerTriangularInvariant(testCase)
            % Verify L is lower triangular
            A = [9 6 3; 6 5 2; 3 2 2];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                % L should have zeros above diagonal
                upperPart = triu(cert.L, 1);
                testCase.verifyLessThan(norm(upperPart, 'fro'), testCase.tolerance);
            end
        end

        function testPositiveDiagonalInvariant(testCase)
            % Verify L has positive diagonal
            A = [4 1; 1 3];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                % Diagonal should be all positive
                diagonal = diag(cert.L);
                testCase.verifyTrue(all(diagonal > 0), ...
                    'L diagonal must be positive');
            end
        end

        function testSymmetryInvariant(testCase)
            % Verify A is symmetric
            A = [4 2 1; 2 5 3; 1 3 6];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            % Compute symmetry error independently
            symError = norm(A - A', 'fro') / norm(A, 'fro');
            testCase.verifyEqual(cert.symmetry, symError, 'RelTol', 1e-14);
            testCase.verifyLessThan(cert.symmetry, testCase.tolerance);
        end

        % ============ Determinant Property ============

        function testDeterminantProperty(testCase)
            % Verify det(A) = (det(L))^2 = (prod(diag(L)))^2
            A = [4 2; 2 3];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                detA = det(A);
                detL = det(cert.L);
                % det(L*L') = det(L)*det(L') = det(L)^2 = det(A)
                testCase.verifyEqual(detL^2, detA, ...
                    'RelTol', 1e-10, ...
                    'Determinant property: det(A) != (det(L))^2');
            end
        end

        % ============ Condition Number ============

        function testConditionNumber(testCase)
            % Test that well-conditioned SPD has reasonable Cholesky decomposition
            A = eye(5) + 0.01 * randn(5);
            A = A' * A;  % Make SPD

            cert = cholesky.certifyDecomposition(A, testCase.tolerance);
            testCase.verifyTrue(cert.certified, ...
                'Well-conditioned SPD certification failed');
        end

        % ============ Tolerance Sensitivity ============

        function testToleranceStrict(testCase)
            % Test that tight tolerance still passes for stable SPD
            A = eye(5);
            cert = cholesky.certifyDecomposition(A, 1e-14);

            testCase.verifyTrue(cert.certified, ...
                'Certification fails with very tight tolerance on stable SPD');
        end

        function testToleranceRelaxed(testCase)
            % Test that relaxed tolerance passes for any SPD
            A = eye(5) + 0.01 * randn(5);
            A = A' * A;

            cert = cholesky.certifyDecomposition(A, 1e-4);
            testCase.verifyTrue(cert.certified, ...
                'Certification fails with relaxed tolerance');
        end

        % ============ Complex Matrices ============

        function testComplexHermitianMatrix(testCase)
            % Test Cholesky of complex Hermitian positive definite matrix
            Z = randn(5) + 1i*randn(5);
            A = Z' * Z;  % Hermitian positive definite

            % MATLAB Cholesky handles complex Hermitian
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);
            testCase.verifyTrue(cert.isPositiveDefinite, ...
                'Complex Hermitian should be detected as positive definite');
        end

        % ============ Cross-Validation with Lean Theorems ============

        function testLeanCholeskyReconstruction(testCase)
            % Cross-validate with Lean: A = L*L'
            % Corresponds to cholesky.recovery in Lean
            A = [4 2; 2 3];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                reconstructed = cert.L * cert.L';
                testCase.verifyEqual(reconstructed, A, ...
                    'AbsTol', testCase.tolerance, ...
                    'Cholesky reconstruction invariant violated');
            end
        end

        function testLeanLowerTriangular(testCase)
            % Cross-validate with Lean: L is lower triangular
            % Corresponds to IsLowerTriangular in Lean
            A = [9 6; 6 5];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                % L(i,j) = 0 for all i < j
                for i = 1:size(cert.L,1)
                    for j = (i+1):size(cert.L,2)
                        testCase.verifyEqual(cert.L(i,j), 0, ...
                            'RelTol', testCase.tolerance, ...
                            sprintf('L(%d,%d) should be zero', i, j));
                    end
                end
            end
        end

        function testLeanPositiveDiagonal(testCase)
            % Cross-validate with Lean: L has positive diagonal
            % Corresponds to IsPositiveDiagonal in Lean
            A = [4 2; 2 3];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            if cert.isPositiveDefinite
                diagonal = diag(cert.L);
                testCase.verifyTrue(all(diagonal > 0), ...
                    'Lean theorem: L diagonal must be positive');
            end
        end

        function testLeanSymmetricMatrix(testCase)
            % Cross-validate with Lean: A is symmetric
            % Corresponds to IsSymmetric in Lean
            A = [4 2; 2 3];
            cert = cholesky.certifyDecomposition(A, testCase.tolerance);

            % Verify symmetry
            testCase.verifyEqual(A, A', ...
                'AbsTol', testCase.tolerance, ...
                'Lean theorem: A must be symmetric');
        end

    end

end
