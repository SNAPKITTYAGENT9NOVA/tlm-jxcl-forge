use ndarray::{Array, IxDyn, ShapeBuilder};

use crate::error::{TensorError, TensorResult};
use crate::tensor::Tensor;

/// Memory layout order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Row-major: the last axis varies fastest (NumPy's default, `C`).
    C,
    /// Column-major: the first axis varies fastest (`F` / Fortran order,
    /// what LAPACK expects).
    Fortran,
}

impl<T: Clone> Tensor<T> {
    /// Reshape into `shape`, without copying when the current layout
    /// permits it (falls back to a copy otherwise, exactly like
    /// `ndarray`'s `to_shape`/`into_shape_clone`).
    pub fn reshape(&self, shape: &[usize]) -> TensorResult<Tensor<T>> {
        let from_len = self.len();
        let to_len: usize = shape.iter().product();
        if from_len != to_len {
            return Err(TensorError::ReshapeError {
                from: self.shape().to_vec(),
                to: shape.to_vec(),
                from_len,
                to_len,
            });
        }
        Ok(Tensor::from_array(
            self.data
                .to_shape(IxDyn(shape))
                .map(|v| v.to_owned())
                .map_err(|_| TensorError::ReshapeError {
                    from: self.shape().to_vec(),
                    to: shape.to_vec(),
                    from_len,
                    to_len,
                })?,
        ))
    }

    /// Return a tensor with the same data, relaid out in the requested
    /// memory order. A no-op copy if already in that order.
    pub fn to_order(&self, order: Order) -> Tensor<T> {
        let shape = self.shape().to_vec();
        match order {
            Order::C => Tensor::from_array(self.data.as_standard_layout().to_owned()),
            Order::Fortran => {
                // Iterating the fully-transposed view in its own standard
                // (C) order visits the original array's elements with the
                // *first* axis fastest -- i.e. Fortran order -- which is
                // exactly the flat buffer `.f()`-shaped construction wants.
                let fortran_order: Vec<T> = self.data.t().iter().cloned().collect();
                Tensor::from_array(Array::from_shape_vec(IxDyn(&shape).f(), fortran_order).unwrap())
            }
        }
    }
}

impl<T> Tensor<T> {
    /// True if the underlying buffer is Fortran- (column-major) ordered.
    pub fn is_fortran_layout(&self) -> bool {
        self.data.t().is_standard_layout()
    }
}
