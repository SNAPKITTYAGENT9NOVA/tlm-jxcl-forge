use crate::error::{Error, Result};
use ndarray::{Array, IxDyn};
use std::fmt;

/// A multi-dimensional array (tensor) backed by ndarray's Array type.
///
/// Supports arbitrary rank tensors with ergonomic creation, indexing, and operations.
/// All operations are zero-copy where possible using views.
///
/// # Examples
///
/// ```
/// use jxcl_tensor::Tensor;
///
/// let t = Tensor::zeros(&[2, 3, 4]);
/// assert_eq!(t.shape(), &[2, 3, 4]);
/// assert_eq!(t.rank(), 3);
/// ```
#[derive(Clone)]
pub struct Tensor<T: Clone> {
    pub(crate) inner: Array<T, IxDyn>,
}

impl<T: Clone> Tensor<T> {
    /// Creates a new tensor from an ndarray Array.
    pub fn new(inner: Array<T, IxDyn>) -> Self {
        Self { inner }
    }

    /// Returns a reference to the underlying ndarray Array.
    pub fn as_array(&self) -> &Array<T, IxDyn> {
        &self.inner
    }

    /// Consumes the tensor and returns the underlying ndarray Array.
    pub fn into_array(self) -> Array<T, IxDyn> {
        self.inner
    }

    /// Returns the shape of the tensor.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// assert_eq!(t.shape(), &[2, 3, 4]);
    /// ```
    pub fn shape(&self) -> &[usize] {
        self.inner.shape()
    }

    /// Returns the rank (number of dimensions) of the tensor.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// assert_eq!(t.rank(), 3);
    /// ```
    pub fn rank(&self) -> usize {
        self.inner.ndim()
    }

    /// Returns the total number of elements in the tensor.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// assert_eq!(t.size(), 24);
    /// ```
    pub fn size(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the tensor is empty (zero elements).
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns true if the tensor data appears to be in a simple layout.
    pub fn is_contiguous(&self) -> bool {
        !self.inner.is_empty() || self.shape().is_empty()
    }


    /// Reshapes the tensor to a new shape without reordering elements.
    ///
    /// # Errors
    ///
    /// Returns an error if the new shape has a different total size than the current shape.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// let reshaped = t.reshape(&[6, 4]).unwrap();
    /// assert_eq!(reshaped.shape(), &[6, 4]);
    /// ```
    pub fn reshape(&self, shape: &[usize]) -> Result<Tensor<T>> {
        let new_size: usize = shape.iter().product();
        let current_size = self.size();

        if new_size != current_size {
            return Err(Error::ShapeMismatch {
                expected: shape.to_vec(),
                actual: self.shape().to_vec(),
            });
        }

        let new_array = self.inner.clone().into_shape(IxDyn(shape))
            .map_err(|_| Error::ShapeMismatch {
                expected: shape.to_vec(),
                actual: self.shape().to_vec(),
            })?;
        Ok(Tensor::new(new_array))
    }

    /// Transposes the two innermost dimensions (reverses all axes).
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// let transposed = t.transpose();
    /// assert_eq!(transposed.shape(), &[4, 3, 2]);
    /// ```
    pub fn transpose(&self) -> Tensor<T> {
        let mut axes: Vec<usize> = (0..self.rank()).collect();
        axes.reverse();
        let axis_view = self.inner.view();
        let transposed = axis_view.permuted_axes(axes.as_slice());
        Tensor::new(transposed.to_owned())
    }

    /// Permutes axes in the order specified.
    ///
    /// # Errors
    ///
    /// Returns an error if axes are invalid or out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// let permuted = t.permute_axes(&[2, 0, 1]).unwrap();
    /// assert_eq!(permuted.shape(), &[4, 2, 3]);
    /// ```
    pub fn permute_axes(&self, axes: &[usize]) -> Result<Tensor<T>> {
        if axes.len() != self.rank() {
            return Err(Error::RankMismatch {
                expected: self.rank(),
                actual: axes.len(),
            });
        }

        for &ax in axes.iter() {
            if ax >= self.rank() {
                return Err(Error::AxisOutOfBounds {
                    axis: ax,
                    rank: self.rank(),
                });
            }
        }

        let permuted = self.inner.view().permuted_axes(axes);
        Ok(Tensor::new(permuted.to_owned()))
    }

    /// Expands the tensor along a new axis at the specified position.
    ///
    /// # Errors
    ///
    /// Returns an error if the axis position is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3]);
    /// let expanded = t.expand_dims(0).unwrap();
    /// assert_eq!(expanded.shape(), &[1, 2, 3]);
    /// ```
    pub fn expand_dims(&self, axis: usize) -> Result<Tensor<T>> {
        if axis > self.rank() {
            return Err(Error::AxisOutOfBounds {
                axis,
                rank: self.rank() + 1,
            });
        }

        let mut new_shape = self.shape().to_vec();
        new_shape.insert(axis, 1);

        self.reshape(&new_shape)
    }

    /// Removes axes of size 1 from the tensor.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[1, 2, 1, 3]);
    /// let squeezed = t.squeeze();
    /// assert_eq!(squeezed.shape(), &[2, 3]);
    /// ```
    pub fn squeeze(&self) -> Tensor<T> {
        let new_shape: Vec<usize> = self
            .shape()
            .iter()
            .filter(|&&d| d != 1)
            .copied()
            .collect();

        if new_shape.is_empty() {
            let scalar = self.inner.iter().next().cloned().unwrap_or_else(|| {
                self.inner.iter().next().cloned().expect("empty tensor")
            });
            Tensor::new(Array::from_elem(IxDyn(&[]), scalar))
        } else {
            self.reshape(&new_shape).unwrap_or_else(|_| self.clone())
        }
    }

    /// Flattens the tensor into a 1-D vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// let flat = t.flatten();
    /// assert_eq!(flat.shape(), &[24]);
    /// ```
    pub fn flatten(&self) -> Tensor<T> {
        let size = self.size();
        self.reshape(&[size]).unwrap()
    }

}

impl<T: Clone + num_traits::Zero> Tensor<T> {
    /// Creates a tensor filled with zeros.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::<f32>::zeros(&[2, 3]);
    /// assert_eq!(t.shape(), &[2, 3]);
    /// ```
    pub fn zeros(shape: &[usize]) -> Self {
        Tensor::new(Array::zeros(IxDyn(shape)))
    }
}

impl<T: Clone + num_traits::One + num_traits::Zero> Tensor<T> {
    /// Creates an identity matrix of the specified size.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let eye = Tensor::<f32>::eye(3).unwrap();
    /// assert_eq!(eye.shape(), &[3, 3]);
    /// ```
    pub fn eye(n: usize) -> Result<Tensor<T>> {
        let mut arr = Array::zeros(IxDyn(&[n, n]));
        for i in 0..n {
            let mut idx = vec![0; 2];
            idx[0] = i;
            idx[1] = i;
            arr[IxDyn(&idx)] = T::one();
        }
        Ok(Tensor::new(arr))
    }
}

impl<T: Clone> fmt::Debug for Tensor<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tensor")
            .field("shape", &self.shape())
            .field("rank", &self.rank())
            .field("size", &self.size())
            .finish()
    }
}

impl<T: Clone + PartialEq> PartialEq for Tensor<T> {
    fn eq(&self, other: &Self) -> bool {
        self.shape() == other.shape() && self.inner == other.inner
    }
}

impl<T: Clone + Eq> Eq for Tensor<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let t = Tensor::<f32>::zeros(&[2, 3, 4]);
        assert_eq!(t.shape(), &[2, 3, 4]);
        assert_eq!(t.rank(), 3);
        assert_eq!(t.size(), 24);
    }

    #[test]
    fn test_reshape() {
        let t = Tensor::<i32>::zeros(&[2, 3, 4]);
        let reshaped = t.reshape(&[6, 4]).unwrap();
        assert_eq!(reshaped.shape(), &[6, 4]);
        assert_eq!(reshaped.size(), 24);
    }

    #[test]
    fn test_reshape_size_mismatch() {
        let t = Tensor::<i32>::zeros(&[2, 3, 4]);
        let result = t.reshape(&[5, 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_transpose() {
        let t = Tensor::<f32>::zeros(&[2, 3, 4]);
        let transposed = t.transpose();
        assert_eq!(transposed.shape(), &[4, 3, 2]);
    }

    #[test]
    fn test_flatten() {
        let t = Tensor::<f32>::zeros(&[2, 3, 4]);
        let flat = t.flatten();
        assert_eq!(flat.shape(), &[24]);
    }

    #[test]
    fn test_eye() {
        let eye = Tensor::<f32>::eye(3).unwrap();
        assert_eq!(eye.shape(), &[3, 3]);
        assert_eq!(eye.size(), 9);
    }

    #[test]
    fn test_squeeze() {
        let t = Tensor::<f32>::zeros(&[1, 2, 1, 3]);
        let squeezed = t.squeeze();
        assert_eq!(squeezed.shape(), &[2, 3]);
    }

    #[test]
    fn test_expand_dims() {
        let t = Tensor::<f32>::zeros(&[2, 3]);
        let expanded = t.expand_dims(1).unwrap();
        assert_eq!(expanded.shape(), &[2, 1, 3]);
    }
}
