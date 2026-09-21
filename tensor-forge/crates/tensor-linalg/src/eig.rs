use ndarray::Array2;
use num_traits::Float;
use tensor_core::Tensor;

use crate::error::{LinAlgError, LinAlgResult};
use crate::util::{require_square, to_array2};

/// An eigendecomposition: `A == V @ diag(values) @ V^T` for symmetric
/// `A`, with `values` sorted descending and `vectors`' columns the
/// corresponding orthonormal eigenvectors.
#[derive(Debug, Clone)]
pub struct EigDecomposition<T> {
    pub values: Tensor<T>,
    pub vectors: Tensor<T>,
}

/// Eigenvalues and eigenvectors of a **symmetric** matrix, via the
/// classical cyclic Jacobi eigenvalue algorithm: repeatedly zero out the
/// largest-magnitude off-diagonal entry with a Givens rotation until the
/// matrix is (numerically) diagonal. Guaranteed to converge for symmetric
/// input; general (non-symmetric, possibly complex-eigenvalue) matrices
/// are out of scope for this reference implementation.
pub fn eig_symmetric<T: Float>(
    a: &Tensor<T>,
    max_sweeps: usize,
    tol: T,
) -> LinAlgResult<EigDecomposition<T>> {
    let mat = to_array2(a, "eig_symmetric")?;
    let n = require_square(&mat, "eig_symmetric")?;

    let sym_tol = T::epsilon() * T::from(10).unwrap();
    for i in 0..n {
        for j in (i + 1)..n {
            let scale = T::one() + mat[[i, j]].abs();
            if (mat[[i, j]] - mat[[j, i]]).abs() > sym_tol * scale {
                return Err(LinAlgError::NotSymmetric {
                    op: "eig_symmetric",
                });
            }
        }
    }

    let mut a = mat;
    let mut v = Array2::<T>::eye(n);
    let mut converged = n <= 1;

    for _ in 0..max_sweeps {
        let mut p = 0;
        let mut q = 1;
        let mut off = T::zero();
        for i in 0..n {
            for j in (i + 1)..n {
                let m = a[[i, j]].abs();
                if m > off {
                    off = m;
                    p = i;
                    q = j;
                }
            }
        }
        if off < tol {
            converged = true;
            break;
        }

        let theta = (a[[q, q]] - a[[p, p]]) / (a[[p, q]] + a[[p, q]]);
        let t = if theta >= T::zero() {
            T::one() / (theta + (theta * theta + T::one()).sqrt())
        } else {
            -T::one() / (-theta + (theta * theta + T::one()).sqrt())
        };
        let c = T::one() / (t * t + T::one()).sqrt();
        let s = t * c;

        let apq = a[[p, q]];
        a[[p, p]] = a[[p, p]] - t * apq;
        a[[q, q]] = a[[q, q]] + t * apq;
        a[[p, q]] = T::zero();
        a[[q, p]] = T::zero();

        for i in 0..n {
            if i != p && i != q {
                let aip = a[[i, p]];
                let aiq = a[[i, q]];
                a[[i, p]] = c * aip - s * aiq;
                a[[p, i]] = a[[i, p]];
                a[[i, q]] = s * aip + c * aiq;
                a[[q, i]] = a[[i, q]];
            }
        }
        for i in 0..n {
            let vip = v[[i, p]];
            let viq = v[[i, q]];
            v[[i, p]] = c * vip - s * viq;
            v[[i, q]] = s * vip + c * viq;
        }
    }

    if !converged {
        return Err(LinAlgError::ConvergenceFailure {
            op: "eig_symmetric",
            max_iter: max_sweeps,
        });
    }

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| a[[j, j]].partial_cmp(&a[[i, i]]).unwrap());

    let values: Vec<T> = order.iter().map(|&i| a[[i, i]]).collect();
    let mut vectors = Array2::<T>::zeros((n, n));
    for (col, &src) in order.iter().enumerate() {
        for row in 0..n {
            vectors[[row, col]] = v[[row, src]];
        }
    }

    Ok(EigDecomposition {
        values: Tensor::from_vec(&[n], values).map_err(LinAlgError::Tensor)?,
        vectors: Tensor::from_owned(vectors),
    })
}
