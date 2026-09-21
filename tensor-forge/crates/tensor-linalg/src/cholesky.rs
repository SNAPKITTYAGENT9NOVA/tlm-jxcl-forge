use ndarray::Array2;
use num_traits::Float;
use tensor_core::Tensor;

use crate::error::{LinAlgError, LinAlgResult};
use crate::util::{require_square, to_array2};

/// The Cholesky decomposition `A = L @ L^T` of a symmetric positive-
/// definite matrix, with `L` lower-triangular.
pub fn cholesky<T: Float>(a: &Tensor<T>) -> LinAlgResult<Tensor<T>> {
    let mat = to_array2(a, "cholesky")?;
    let n = require_square(&mat, "cholesky")?;

    let sym_tol = T::epsilon() * T::from(10).unwrap();
    for i in 0..n {
        for j in (i + 1)..n {
            let scale = T::one() + mat[[i, j]].abs();
            if (mat[[i, j]] - mat[[j, i]]).abs() > sym_tol * scale {
                return Err(LinAlgError::NotSymmetric { op: "cholesky" });
            }
        }
    }

    let mut l = Array2::<T>::zeros((n, n));
    for i in 0..n {
        for j in 0..=i {
            let mut sum = mat[[i, j]];
            for k in 0..j {
                sum = sum - l[[i, k]] * l[[j, k]];
            }
            if i == j {
                if sum <= T::zero() {
                    return Err(LinAlgError::NotPositiveDefinite { op: "cholesky" });
                }
                l[[i, j]] = sum.sqrt();
            } else {
                l[[i, j]] = sum / l[[j, j]];
            }
        }
    }
    Ok(Tensor::from_owned(l))
}
