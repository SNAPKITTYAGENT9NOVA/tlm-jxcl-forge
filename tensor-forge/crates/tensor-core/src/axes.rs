use ndarray::{Axis, IxDyn};

use crate::error::{TensorError, TensorResult};
use crate::tensor::Tensor;

impl<T: Clone> Tensor<T> {
    /// Reverse all axes (the generalized transpose). For a 2-D tensor this
    /// is the ordinary matrix transpose.
    pub fn transpose(&self) -> Tensor<T> {
        Tensor::from_array(self.data.t().to_owned())
    }

    /// Reorder axes according to `axes`, a permutation of `0..self.ndim()`
    /// (like NumPy's `transpose(axes)` / `permute`).
    pub fn permute(&self, axes: &[usize]) -> TensorResult<Tensor<T>> {
        let ndim = self.ndim();
        if axes.len() != ndim {
            return Err(TensorError::RankMismatch {
                expected: ndim,
                actual: axes.len(),
            });
        }
        let mut seen = vec![false; ndim];
        for &a in axes {
            if a >= ndim || std::mem::replace(&mut seen[a], true) {
                return Err(TensorError::InvalidPattern(format!(
                    "permute axes {axes:?} is not a permutation of 0..{ndim}"
                )));
            }
        }
        Ok(Tensor::from_array(
            self.data.clone().permuted_axes(IxDyn(axes)),
        ))
    }

    /// Swap two axes.
    pub fn swap_axes(&self, a: usize, b: usize) -> TensorResult<Tensor<T>> {
        let ndim = self.ndim();
        if a >= ndim || b >= ndim {
            return Err(TensorError::AxisOutOfBounds {
                axis: a.max(b),
                ndim,
            });
        }
        let mut out = self.data.clone();
        out.swap_axes(a, b);
        Ok(Tensor::from_array(out))
    }

    /// Insert a new length-1 axis at position `axis`.
    pub fn insert_axis(&self, axis: usize) -> TensorResult<Tensor<T>> {
        if axis > self.ndim() {
            return Err(TensorError::AxisOutOfBounds {
                axis,
                ndim: self.ndim(),
            });
        }
        Ok(Tensor::from_array(
            self.data.clone().insert_axis(Axis(axis)),
        ))
    }

    /// Remove a length-1 axis at position `axis`. Fails if that axis's
    /// length is not 1.
    pub fn squeeze_axis(&self, axis: usize) -> TensorResult<Tensor<T>> {
        let ndim = self.ndim();
        if axis >= ndim {
            return Err(TensorError::AxisOutOfBounds { axis, ndim });
        }
        if self.shape()[axis] != 1 {
            return Err(TensorError::ShapeMismatch {
                expected: vec![1],
                actual: vec![self.shape()[axis]],
            });
        }
        Ok(Tensor::from_array(
            self.data.clone().remove_axis(Axis(axis)),
        ))
    }

    /// Remove every length-1 axis.
    pub fn squeeze(&self) -> Tensor<T> {
        let mut out = self.data.clone();
        // Iterate original axis indices high-to-low: removing a
        // higher-indexed axis never shifts the position of a
        // lower-indexed one, so checking `self.shape()` (fixed) against
        // `out` (shrinking) stays in sync.
        for axis in (0..self.ndim()).rev() {
            if self.shape()[axis] == 1 {
                out = out.remove_axis(Axis(axis));
            }
        }
        Tensor::from_array(out)
    }
}
