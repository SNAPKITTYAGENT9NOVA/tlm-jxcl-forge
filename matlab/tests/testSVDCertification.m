classdef testSVDCertification < matlab.unittest.TestCase
    % Comprehensive test suite for SVD certification
    % Tests correspondence with Lean formalization theorems

    properties
        tolerance = 1e-10
    end

    methods(Test)

        % ============ Full-Rank Matrices ============

        function testSquareMatrix(testCase)
            % Test SVD of 5x5 square matrix
            A = randn(5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Square matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testTallMatrix(testCase)
            % Test SVD of 10x5 tall matrix
            A = randn(10, 5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Tall matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testWideMatrix(testCase)
            % Test SVD of 5x10 wide matrix
            A = randn(5, 10);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Wide matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        % ============ Special Matrices ============

        function testIdentityMatrix(testCase)
            % Test SVD of identity matrix
            A = eye(5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Identity matrix certification failed');
            % All singular values should be 1
            singularValues = diag(cert.S);
            testCase.verifyLessThan(...
                norm(singularValues - 1), testCase.tolerance);
        end

        function testRankDeficient(testCase)
            % Test SVD of rank-deficient matrix
            % Rank 2 matrix in 5x5
            A = randn(5, 2) * randn(2, 5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Rank-deficient matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        function testDiagonalMatrix(testCase)
            % Test SVD of diagonal matrix
            A = diag([5, 3, 1, 2, 4]);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Diagonal matrix certification failed');
            % Singular values should be absolute values of diagonal entries
            expectedSingularValues = sort([5, 3, 1, 2, 4], 'descend')';
            actualSingularValues = sort(diag(cert.S), 'descend');
            testCase.verifyEqual(actualSingularValues, expectedSingularValues, ...
                'RelTol', testCase.tolerance);
        end

        % ============ Numerical Stability ============

        function testIllConditioned(testCase)
            % Test SVD of ill-conditioned matrix
            A = gallery('hilb', 10);  % Hilbert matrix (very ill-conditioned)
            cert = svd.certifyDecomposition(A, 1e-8);  % Relax tolerance

            testCase.verifyTrue(cert.certified, ...
                'Ill-conditioned matrix certification failed');
        end

        function testSmallMatrix(testCase)
            % Test SVD of very small values
            A = 1e-15 * randn(5);
            cert = svd.certifyDecomposition(A, 1e-10);

            testCase.verifyTrue(cert.certified, ...
                'Small value matrix certification failed');
        end

        function testLargeMatrix(testCase)
            % Test SVD of large matrix
            A = 1e10 * randn(5);
            cert = svd.certifyDecomposition(A, 1e-8);

            testCase.verifyTrue(cert.certified, ...
                'Large value matrix certification failed');
        end

        % ============ Individual Invariants ============

        function testReconstructionInvariant(testCase)
            % Verify reconstruction error ||A - U*S*V'|| is within tolerance
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            % Compute reconstruction independently
            reconError = norm(A - cert.U*cert.S*cert.V', 'fro') / norm(A, 'fro');

            testCase.verifyLessThan(reconError, testCase.tolerance);
            testCase.verifyEqual(cert.reconstruction, reconError, ...
                'RelTol', 1e-14);
        end

        function testOrthogonalityU(testCase)
            % Verify U is orthogonal: U'*U = I
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            [m, n] = size(A);
            minDim = min(m, n);
            U_square = cert.U(:, 1:minDim);

            % Check U'*U = I
            UtU = U_square' * U_square;
            I_expected = eye(minDim);
            testCase.verifyEqual(UtU, I_expected, ...
                'AbsTol', testCase.tolerance, ...
                'U orthogonality violated: U''*U != I');
        end

        function testOrthogonalityV(testCase)
            % Verify V is orthogonal: V'*V = I
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            [m, n] = size(A);

            % Check V'*V = I
            VtV = cert.V' * cert.V;
            I_expected = eye(n);
            testCase.verifyEqual(VtV, I_expected, ...
                'AbsTol', testCase.tolerance, ...
                'V orthogonality violated: V''*V != I');
        end

        function testDiagonalSingularValues(testCase)
            % Verify S is diagonal with non-negative singular values
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            % Check S is diagonal (off-diagonal elements are zero)
            offDiag = cert.S - diag(diag(cert.S));
            testCase.verifyLessThan(norm(offDiag, 'fro'), testCase.tolerance);

            % Check singular values are non-negative
            singularValues = diag(cert.S);
            testCase.verifyTrue(all(singularValues >= 0), ...
                'Singular values must be non-negative');
        end

        function testSingularValuesDecreasing(testCase)
            % Verify singular values are in decreasing order
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            singularValues = diag(cert.S);
            for i = 1:(length(singularValues)-1)
                testCase.verifyGreaterThanOrEqual(singularValues(i), ...
                    singularValues(i+1), ...
                    sprintf('Singular values not decreasing: s_%d < s_%d', ...
                    i, i+1));
            end
        end

        % ============ Determinant and Rank ============

        function testDeterminantFromSVD(testCase)
            % Verify det(A) = product of singular values (with sign)
            A = [2 1; 3 4];  % Simple 2x2 for clear singular values
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            detA = det(A);
            singularProduct = prod(diag(cert.S));

            % det(A) = ±(product of singular values)
            testCase.verifyEqual(abs(detA), singularProduct, ...
                'RelTol', 1e-10, ...
                'Determinant should equal product of singular values');
        end

        function testRankFromSVD(testCase)
            % Verify rank matches number of non-zero singular values
            A = randn(10, 5);  % Always rank 5
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            singularValues = diag(cert.S);
            numNonZero = sum(abs(singularValues) > testCase.tolerance);

            expectedRank = rank(A);
            testCase.verifyEqual(numNonZero, expectedRank, ...
                'Rank mismatch: number of non-zero singular values');
        end

        % ============ Tolerance Sensitivity ============

        function testToleranceStrict(testCase)
            % Test that tight tolerance still passes for stable matrices
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, 1e-14);

            testCase.verifyTrue(cert.certified, ...
                'Certification fails with very tight tolerance on stable matrix');
        end

        function testToleranceRelaxed(testCase)
            % Test that relaxed tolerance passes for any matrix
            A = randn(6, 6);
            cert = svd.certifyDecomposition(A, 1e-4);

            testCase.verifyTrue(cert.certified, ...
                'Certification fails with relaxed tolerance');
        end

        % ============ Complex Matrices ============

        function testComplexMatrix(testCase)
            % Test SVD of complex-valued matrix
            A = randn(5) + 1i*randn(5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            testCase.verifyTrue(cert.certified, ...
                'Complex matrix certification failed');
            testCase.verifyLessThan(cert.reconstruction, testCase.tolerance);
        end

        % ============ Cross-Validation with Lean Theorems ============

        function testLeanSVDReconstruction(testCase)
            % Cross-validate with Lean: A = U*S*V'
            % Corresponds to svd.recovery in Lean
            A = [1 2 3; 4 5 6];  % Simple 2x3 matrix
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            reconstructed = cert.U * cert.S * cert.V';
            testCase.verifyEqual(reconstructed, A, ...
                'AbsTol', testCase.tolerance, ...
                'SVD reconstruction invariant violated');
        end

        function testLeanOrthogonalityU(testCase)
            % Cross-validate with Lean: U is orthogonal (U'*U = I)
            A = randn(5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            [m, n] = size(A);
            minDim = min(m, n);
            U_square = cert.U(:, 1:minDim);
            UtU = U_square' * U_square;

            testCase.verifyEqual(UtU, eye(minDim), ...
                'AbsTol', testCase.tolerance, ...
                'Lean theorem: U is orthogonal violated');
        end

        function testLeanOrthogonalityV(testCase)
            % Cross-validate with Lean: V is orthogonal (V'*V = I)
            A = randn(5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            [m, n] = size(A);
            VtV = cert.V' * cert.V;

            testCase.verifyEqual(VtV, eye(n), ...
                'AbsTol', testCase.tolerance, ...
                'Lean theorem: V is orthogonal violated');
        end

        function testLeanSingularValueProperty(testCase)
            % Cross-validate with Lean: S is diagonal with non-negative singular values
            A = randn(5);
            cert = svd.certifyDecomposition(A, testCase.tolerance);

            % S should be diagonal
            offDiag = cert.S - diag(diag(cert.S));
            testCase.verifyLessThan(norm(offDiag, 'fro'), testCase.tolerance, ...
                'Lean theorem: S is diagonal violated');

            % All singular values non-negative
            singularValues = diag(cert.S);
            testCase.verifyTrue(all(singularValues >= -testCase.tolerance), ...
                'Lean theorem: singular values must be non-negative');
        end

    end

end
