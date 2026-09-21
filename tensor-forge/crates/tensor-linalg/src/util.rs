use ndarray::{Array1, Array2, Ix1, Ix2};
use tensor_core::Tensor;

use crate::error::{LinAlgError, LinAlgResult};

pub(crate) fn to_array2<T: Clone>(t: &Tensor<T>, op: &'static str) -> LinAlgResult<Array2<T>> {
    let ndim = t.ndim();
    t.clone()
        .into_dimensionality::<Ix2>()
        .map_err(|_| LinAlgError::RankMismatch {
            op,
            expected: 2,
            actual: ndim,
        })
}

pub(crate) fn to_array1<T: Clone>(t: &Tensor<T>, op: &'static str) -> LinAlgResult<Array1<T>> {
    let ndim = t.ndim();
    t.clone()
        .into_dimensionality::<Ix1>()
        .map_err(|_| LinAlgError::RankMismatch {
            op,
            expected: 1,
            actual: ndim,
        })
}

pub(crate) fn require_square<T>(mat: &Array2<T>, op: &'static str) -> LinAlgResult<usize> {
    let (r, c) = mat.dim();
    if r != c {
        return Err(LinAlgError::NotSquare {
            op,
            shape: vec![r, c],
        });
    }
    Ok(r)
}
