use ndarray::{SliceInfo, SliceInfoElem};

use crate::error::{TensorError, TensorResult};
use crate::ops::broadcast_shape;
use crate::tensor::Tensor;

/// One axis of a runtime-built slice, as passed to [`Tensor::slice_dyn`].
///
/// Unlike `ndarray`'s `s![]` macro (which needs the axis count known at
/// compile time), `SliceSpec` lists are built and sized at runtime, which
/// is what a tensor of runtime-only-known rank needs. Negative `start`/
/// `end`/`index` values count from the end of the axis, exactly like
/// NumPy (`-1` is the last element).
#[derive(Debug, Clone, Copy)]
pub enum SliceSpec {
    /// Select a single index along this axis, dropping the axis from the
    /// result (like `arr[i]` in NumPy).
    Index(isize),
    /// A `start..end` range with a step, keeping the axis (like
    /// `arr[start:end:step]`). `end = None` means "through the end" (or
    /// "through the start", when `step` is negative).
    Range {
        start: Option<isize>,
        end: Option<isize>,
        step: isize,
    },
}

impl SliceSpec {
    /// The full axis, unsliced.
    pub fn full() -> Self {
        SliceSpec::Range {
            start: None,
            end: None,
            step: 1,
        }
    }

    fn to_slice_info_elem(self) -> TensorResult<SliceInfoElem> {
        match self {
            SliceSpec::Index(i) => Ok(SliceInfoElem::Index(i)),
            SliceSpec::Range { start, end, step } => {
                if step == 0 {
                    return Err(TensorError::InvalidStep);
                }
                Ok(SliceInfoElem::Slice {
                    start: start.unwrap_or(0),
                    end,
                    step,
                })
            }
        }
    }
}

impl<T: Clone> Tensor<T> {
    /// Slice the tensor along every axis at once, using a runtime-sized
    /// list of [`SliceSpec`]s (one per axis). Supports negative indices
    /// and negative/positive steps on every axis, which is what makes
    /// this useful over `ndarray`'s `s![]` macro for a tensor whose rank
    /// isn't known until runtime.
    pub fn slice_dyn(&self, specs: &[SliceSpec]) -> TensorResult<Tensor<T>> {
        if specs.len() != self.ndim() {
            return Err(TensorError::RankMismatch {
                expected: specs.len(),
                actual: self.ndim(),
            });
        }
        let elems: Vec<SliceInfoElem> = specs
            .iter()
            .map(|s| s.to_slice_info_elem())
            .collect::<TensorResult<_>>()?;
        let info: SliceInfo<_, ndarray::IxDyn, ndarray::IxDyn> =
            SliceInfo::try_from(elems).map_err(|e| TensorError::InvalidPattern(e.to_string()))?;
        Ok(Tensor::from_array(self.data.slice(info).to_owned()))
    }

    /// Broadcast (zero-copy, as a view materialized into an owned tensor)
    /// to `shape`. Fails if `self.shape()` cannot be broadcast to `shape`
    /// under NumPy's broadcasting rules.
    pub fn broadcast_to(&self, shape: &[usize]) -> TensorResult<Tensor<T>> {
        if broadcast_shape(self.shape(), shape).as_deref() != Some(shape) {
            return Err(TensorError::BroadcastError {
                from: self.shape().to_vec(),
                to: shape.to_vec(),
            });
        }
        self.data
            .broadcast(shape)
            .map(|v| Tensor::from_array(v.to_owned()))
            .ok_or_else(|| TensorError::BroadcastError {
                from: self.shape().to_vec(),
                to: shape.to_vec(),
            })
    }
}
