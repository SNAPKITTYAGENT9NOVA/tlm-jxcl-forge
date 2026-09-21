use ndarray::{Array, ArrayD, ArrayViewD, ArrayViewMutD, Dimension, IxDyn};

use crate::error::{TensorError, TensorResult};

/// An arbitrary-rank, owned tensor.
///
/// `Tensor<T>` is a thin, ergonomic newtype around [`ndarray::ArrayD<T>`]
/// (i.e. `Array<T, IxDyn>`): every tensor, regardless of how it was built,
/// carries its rank at runtime rather than in its type. This is the right
/// default for a general-purpose tensor API (shapes are frequently only
/// known at runtime -- read from a file, computed from another tensor,
/// etc.) while staying zero-cost to convert to and from ndarray's
/// statically-ranked `Array<T, D>` when the rank *is* known at compile
/// time (see [`Tensor::into_dimensionality`] and the `From` impls below).
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor<T> {
    pub(crate) data: ArrayD<T>,
}

impl<T> Tensor<T> {
    /// Wrap an existing dynamic-rank ndarray array with no copying.
    pub fn from_array(data: ArrayD<T>) -> Self {
        Self { data }
    }

    /// The shape of the tensor, outermost axis first.
    pub fn shape(&self) -> &[usize] {
        self.data.shape()
    }

    /// The number of axes (the tensor's rank).
    pub fn ndim(&self) -> usize {
        self.data.ndim()
    }

    /// The total number of elements.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The per-axis strides, in elements (not bytes), as ndarray reports
    /// them: negative for axes stored back-to-front.
    pub fn strides(&self) -> &[isize] {
        self.data.strides()
    }

    /// A read-only, zero-copy view of the whole tensor.
    pub fn view(&self) -> ArrayViewD<'_, T> {
        self.data.view()
    }

    /// A mutable, zero-copy view of the whole tensor.
    pub fn view_mut(&mut self) -> ArrayViewMutD<'_, T> {
        self.data.view_mut()
    }

    /// Borrow the underlying `ndarray` array.
    pub fn as_array(&self) -> &ArrayD<T> {
        &self.data
    }

    /// Borrow the underlying `ndarray` array mutably.
    pub fn as_array_mut(&mut self) -> &mut ArrayD<T> {
        &mut self.data
    }

    /// Consume the tensor, returning the underlying `ndarray` array.
    pub fn into_array(self) -> ArrayD<T> {
        self.data
    }

    /// Attempt to convert into a statically-ranked `ndarray::Array<T, D>`,
    /// e.g. `tensor.into_dimensionality::<Ix2>()`. Fails if the tensor's
    /// runtime rank does not match `D`'s compile-time rank.
    pub fn into_dimensionality<D: Dimension>(self) -> TensorResult<Array<T, D>> {
        let ndim = self.ndim();
        self.data
            .into_dimensionality::<D>()
            .map_err(|_| TensorError::RankMismatch {
                expected: D::NDIM.unwrap_or(ndim),
                actual: ndim,
            })
    }

    /// Checked element access by a runtime index tuple. Returns `None` if
    /// `index.len() != self.ndim()` or any component is out of bounds.
    pub fn get(&self, index: &[usize]) -> Option<&T> {
        if index.len() != self.ndim() {
            return None;
        }
        self.data.get(IxDyn(index))
    }

    /// Checked mutable element access by a runtime index tuple.
    pub fn get_mut(&mut self, index: &[usize]) -> Option<&mut T> {
        if index.len() != self.ndim() {
            return None;
        }
        self.data.get_mut(IxDyn(index))
    }

    /// True if the tensor's data is laid out row-major (C order) with no
    /// gaps, i.e. it can be reinterpreted as a flat slice via
    /// [`ndarray::ArrayBase::as_slice`].
    pub fn is_standard_layout(&self) -> bool {
        self.data.is_standard_layout()
    }
}

impl<T: Clone> Tensor<T> {
    /// Whether the tensor owns a contiguous buffer in *some* order (either
    /// C or Fortran), i.e. a flat view exists without copying.
    pub fn is_contiguous(&self) -> bool {
        self.data.as_slice_memory_order().is_some()
    }

    /// Return the tensor's elements as a flat `Vec`, copying only if the
    /// current layout is not already a contiguous, standard (C) order
    /// buffer.
    pub fn to_vec(&self) -> Vec<T> {
        match self.data.as_slice() {
            Some(slice) => slice.to_vec(),
            None => self.data.iter().cloned().collect(),
        }
    }
}

impl<T> From<ArrayD<T>> for Tensor<T> {
    fn from(data: ArrayD<T>) -> Self {
        Self { data }
    }
}

impl<T> Tensor<T> {
    /// Build a tensor from any statically-ranked `ndarray::Array<T, D>`
    /// (a blanket `From<Array<T, D>>` would conflict with
    /// `From<ArrayD<T>>` when `D = IxDyn`, so this is a plain method
    /// instead).
    pub fn from_owned<D: Dimension>(data: Array<T, D>) -> Self {
        Self {
            data: data.into_dyn(),
        }
    }
}

impl<T> AsRef<ArrayD<T>> for Tensor<T> {
    fn as_ref(&self) -> &ArrayD<T> {
        &self.data
    }
}

impl<T> std::ops::Index<&[usize]> for Tensor<T> {
    type Output = T;

    fn index(&self, index: &[usize]) -> &T {
        &self.data[IxDyn(index)]
    }
}

impl<T> std::ops::IndexMut<&[usize]> for Tensor<T> {
    fn index_mut(&mut self, index: &[usize]) -> &mut T {
        &mut self.data[IxDyn(index)]
    }
}
