classdef testLUCertification < matlab.unittest.TestCase
    % Comprehensive test suite for LU decomposition certification
    % Tests correspondence with Lean formalization theorems

    properties
        tolerance = 1e-10
    end

    methods(Test)

        % ============ Full-Rank Matrices ============

        function testSquareMatrix(testCase)
            % Test LU of 5x5 square matrix
            A = [4 3 2; 1 2 3; 5 1 2];
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Square matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testTallMatrix(testCase)
            % Test LU of 10x5 tall matrix
            A = randn(10, 5);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Tall matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testWideMatrix(testCase)
            % Test LU of 5x10 wide matrix
            A = randn(5, 10);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Wide matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        % ============ Special Matrices ============

        function testIdentityMatrix(testCase)
            % Test LU of identity matrix
            A = eye(5);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Identity matrix certification failed');
            % For identity: L = I, U = I, P = I
            testCase.verifyLessThan(norm(cert.L - eye(5), 'fro'), testCase.tolerance);
            testCase.verifyLessThan(norm(cert.U - eye(5), 'fro'), testCase.tolerance);
        end

        function testTriangularMatrix(testCase)
            % Test LU of upper triangular matrix
            A = triu(randn(5));
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Triangular matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testDiagonalMatrix(testCase)
            % Test LU of diagonal matrix
            A = diag([1, 2, 3, 4, 5]);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Diagonal matrix certification failed');
            % Diagonal should decompose with L=I, U=A
        end

        % ============ Numerical Stability ============

        function testIllConditioned(testCase)
            % Test LU of ill-conditioned matrix
            A = gallery('hilb', 10);  % Hilbert matrix (very ill-conditioned)
            cert = lu.certifyDecomposition(A, 1e-8);  % Relax tolerance

            % Certification should succeed but with larger errors
            testCase.verifyTrue(cert.certified, ...
                'Ill-conditioned matrix certification failed');
        end

        function testSmallMatrix(testCase)
            % Test LU of very small values
            A = 1e-15 * randn(5);
            cert = lu.certifyDecomposition(A, 1e-10);

            testCase.verifyTrue(cert.certified, ...
                'Small value matrix certification failed');
        end

        function testLargeMatrix(testCase)
            % Test LU of large matrix
            A = 1e10 * randn(5);
            cert = lu.certifyDecomposition(A, 1e-8);

            testCase.verifyTrue(cert.certified, ...
                'Large value matrix certification failed');
        end

        % ============ Individual Invariants ============

        function testReconstructionInvariant(testCase)
            % Verify reconstruction error ||P*A - L*U|| is within tolerance
            A = randn(6, 6);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % Compute reconstruction error independently
            P_inv = inv(cert.P);  % MATLAB lu() returns P such that P*A = L*U
            reconError = norm(cert.P*A - cert.L*cert.U, 'fro') / norm(A, 'fro');

            testCase.verifyLessThan(reconError, testCase.tolerance);
            testCase.verifyEqual(cert.reconstruction, reconError, ...
                'RelTol', 1e-14);
        end

        function testLowerTriangularInvariant(testCase)
            % Verify L is lower triangular
            A = randn(6, 6);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % L should have zeros above diagonal
            upperPart = triu(cert.L, 1);
            testCase.verifyLessThan(norm(upperPart, 'fro'), testCase.tolerance);
        end

        function testUpperTriangularInvariant(testCase)
            % Verify U is upper triangular
            A = randn(6, 6);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % U should have zeros below diagonal
            lowerPart = tril(cert.U, -1);
            testCase.verifyLessThan(norm(lowerPart, 'fro'), testCase.tolerance);
        end

        function testUnitDiagonalInvariant(testCase)
            % Verify L has unit diagonal
            A = randn(6, 6);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % L diagonal should be all ones
            diagonal = diag(cert.L);
            testCase.verifyLessThan(norm(diagonal - 1), testCase.tolerance);
        end

        % ============ Determinant Property ============

        function testDeterminantProperty(testCase)
            % Verify det(L)*det(U)*det(P) = det(A)
            % Since L has unit diagonal: det(L) = 1
            % So: det(U)*det(P) = det(A)
            A = randn(6, 6);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            detA = det(A);
            detU = det(cert.U);
            detP = det(cert.P);

            % det(L) = 1 (unit diagonal), so det(L)*det(U)*det(P) = det(U)*det(P)
            expectedDet = detU * detP;

            testCase.verifyEqual(expectedDet, detA, ...
                'RelTol', 1e-10, ...
                'Determinant property fails: det(A) != det(L)*det(U)*det(P)');
        end

        % ============ Tolerance Sensitivity ============

        function testToleranceStrict(testCase)
            % Test that tight tolerance still passes for stable matrices
            A = gallery('randn', [6, 6]);
            cert = lu.certifyDecomposition(A, 1e-14);

            testCase.verifyTrue(cert.certified, ...
                'Certification fails with very tight tolerance on stable matrix');
        end

        function testToleranceRelaxed(testCase)
            % Test that relaxed tolerance passes for any matrix
            A = randn(6, 6);
            cert = lu.certifyDecomposition(A, 1e-4);

            testCase.verifyTrue(cert.certified, ...
                'Certification fails with relaxed tolerance');
        end

        % ============ Complex Matrices ============

        function testComplexMatrix(testCase)
            % Test LU of complex-valued matrix
            A = randn(5) + 1i*randn(5);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Complex matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        % ============ Cross-Validation with Lean Theorems ============

        function testLeanLUReconstruction(testCase)
            % Cross-validate with Lean: A = L*U (up to permutation)
            % Corresponds to lu.recovery in Lean
            A = [1 2; 3 4];  % Simple 2x2 matrix for clarity
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % Verify P*A = L*U
            reconstructed = cert.P * A;
            luProduct = cert.L * cert.U;

            testCase.verifyEqual(reconstructed, luProduct, ...
                'AbsTol', testCase.tolerance, ...
                'LU reconstruction invariant violated');
        end

        function testLeanLowerTriangular(testCase)
            % Cross-validate with Lean: L is lower triangular
            % Corresponds to IsLowerTriangular in Lean
            A = randn(5);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % L(i,j) = 0 for all i < j
            for i = 1:size(cert.L,1)
                for j = (i+1):size(cert.L,2)
                    testCase.verifyEqual(cert.L(i,j), 0, ...
                        'RelTol', testCase.tolerance, ...
                        sprintf('L(%d,%d) should be zero', i, j));
                end
            end
        end

        function testLeanUnitDiagonal(testCase)
            % Cross-validate with Lean: L has unit diagonal
            % Corresponds to IsUnitDiagonal in Lean
            A = randn(5);
            cert = lu.certifyDecomposition(A, testCase.tolerance);

            % L(i,i) = 1 for all i
            diagonal = diag(cert.L);
            testCase.verifyEqual(diagonal, ones(size(diagonal)), ...
                'RelTol', testCase.tolerance, ...
                'L unit diagonal property violated');
        end

    end

end
