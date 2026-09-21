use ndarray::Array2;
use num_traits::Float;
use tensor_core::Tensor;

use crate::error::{LinAlgError, LinAlgResult};
use crate::util::{require_square, to_array1, to_array2};

/// An LU decomposition with partial pivoting: `P @ A == L @ U`, with `L`
/// unit lower-triangular, `U` upper-triangular, and `P` a permutation
/// matrix.
#[derive(Debug, Clone)]
pub struct LuDecomposition<T> {
    pub l: Tensor<T>,
    pub u: Tensor<T>,
    pub p: Tensor<T>,
    /// `perm[i]` is the row of the original matrix now at row `i` of `P @ A`.
    pub perm: Vec<usize>,
    pub num_row_swaps: usize,
}

/// Decompose `a` (an `n x n` matrix) into `P A = L U` via Gaussian
/// elimination with partial pivoting. Fails with
/// [`LinAlgError::SingularMatrix`] if a pivot is (numerically) zero.
pub fn lu<T: Float>(a: &Tensor<T>) -> LinAlgResult<LuDecomposition<T>> {
    let mat = to_array2(a, "lu")?;
    let n = require_square(&mat, "lu")?;

    let mut u = mat;
    let mut l = Array2::<T>::eye(n);
    let mut perm: Vec<usize> = (0..n).collect();
    let mut num_row_swaps = 0usize;

    for k in 0..n {
        let mut pivot_row = k;
        let mut pivot_val = u[[k, k]].abs();
        for i in (k + 1)..n {
            let v = u[[i, k]].abs();
            if v > pivot_val {
                pivot_val = v;
                pivot_row = i;
            }
        }
        if pivot_val <= T::epsilon() {
            return Err(LinAlgError::SingularMatrix { op: "lu" });
        }
        if pivot_row != k {
            for j in 0..n {
                u.swap((k, j), (pivot_row, j));
            }
            for j in 0..k {
                l.swap((k, j), (pivot_row, j));
            }
            perm.swap(k, pivot_row);
            num_row_swaps += 1;
        }
        for i in (k + 1)..n {
            let factor = u[[i, k]] / u[[k, k]];
            l[[i, k]] = factor;
            for j in k..n {
                let delta = factor * u[[k, j]];
                u[[i, j]] = u[[i, j]] - delta;
            }
        }
    }

    let mut p = Array2::<T>::zeros((n, n));
    for (i, &pi) in perm.iter().enumerate() {
        p[[i, pi]] = T::one();
    }

    Ok(LuDecomposition {
        l: Tensor::from_owned(l),
        u: Tensor::from_owned(u),
        p: Tensor::from_owned(p),
        perm,
        num_row_swaps,
    })
}

/// The determinant, via LU decomposition (`det(A) = (-1)^swaps * prod(diag(U))`).
pub fn det<T: Float>(a: &Tensor<T>) -> LinAlgResult<T> {
    match lu(a) {
        Ok(decomp) => {
            let u = to_array2(&decomp.u, "det")?;
            let n = u.nrows();
            let mut result = if decomp.num_row_swaps % 2 == 0 {
                T::one()
            } else {
                -T::one()
            };
            for i in 0..n {
                result = result * u[[i, i]];
            }
            Ok(result)
        }
        Err(LinAlgError::SingularMatrix { .. }) => Ok(T::zero()),
        Err(e) => Err(e),
    }
}

/// Solve `A x = b` for `x`, via forward/back substitution on the LU
/// decomposition of `A`.
pub fn solve<T: Float>(a: &Tensor<T>, b: &Tensor<T>) -> LinAlgResult<Tensor<T>> {
    let decomp = lu(a)?;
    let n = decomp.perm.len();
    let rhs = to_array1(b, "solve")?;
    if rhs.len() != n {
        return Err(LinAlgError::IncompatibleShapes {
            op: "solve",
            lhs: vec![n, n],
            rhs: vec![rhs.len()],
        });
    }
    let l = to_array2(&decomp.l, "solve")?;
    let u = to_array2(&decomp.u, "solve")?;

    // Permute b: pb[i] = b[perm[i]].
    let pb: Vec<T> = decomp.perm.iter().map(|&pi| rhs[pi]).collect();

    // Forward substitution: L y = P b (L is unit lower-triangular).
    let mut y = vec![T::zero(); n];
    for i in 0..n {
        let mut sum = pb[i];
        for j in 0..i {
            sum = sum - l[[i, j]] * y[j];
        }
        y[i] = sum;
    }

    // Back substitution: U x = y.
    let mut x = vec![T::zero(); n];
    for i in (0..n).rev() {
        let mut sum = y[i];
        for j in (i + 1)..n {
            sum = sum - u[[i, j]] * x[j];
        }
        x[i] = sum / u[[i, i]];
    }

    Tensor::from_vec(&[n], x).map_err(LinAlgError::Tensor)
}

/// The matrix inverse, via [`solve`] against each column of the identity.
pub fn inverse<T: Float>(a: &Tensor<T>) -> LinAlgResult<Tensor<T>> {
    let mat = to_array2(a, "inverse")?;
    let n = require_square(&mat, "inverse")?;
    let mut cols = Vec::with_capacity(n);
    for j in 0..n {
        let mut e_j = vec![T::zero(); n];
        e_j[j] = T::one();
        let e_j = Tensor::from_vec(&[n], e_j).map_err(LinAlgError::Tensor)?;
        cols.push(solve(a, &e_j)?.to_vec());
    }
    // `cols[j]` is column j of the inverse; assemble row-major.
    let mut data = vec![T::zero(); n * n];
    for (j, col) in cols.iter().enumerate() {
        for (i, &v) in col.iter().enumerate() {
            data[i * n + j] = v;
        }
    }
    Tensor::from_vec(&[n, n], data).map_err(LinAlgError::Tensor)
}
