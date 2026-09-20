use crate::tensor::Tensor;
use crate::error::{Error, Result};
use ndarray::Axis;
use num_traits::Zero;

/// Reduction operations along axes.
impl<T: Clone> Tensor<T> {
    /// Returns the sum of all elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t: Tensor<i32> = Tensor::new(ndarray::Array::from_shape_vec(
    ///     ndarray::IxDyn(&[2, 3]),
    ///     vec![1, 2, 3, 4, 5, 6],
    /// ).unwrap());
    /// ```
    pub fn sum(&self) -> Result<T>
    where
        T: Zero + std::ops::Add<Output = T>,
    {
        Ok(self
            .inner
            .iter()
            .cloned()
            .fold(T::zero(), |acc, x| acc + x))
    }

    /// Returns the sum along the specified axis, reducing that dimension.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3, 4]);
    /// let summed = t.sum_axis(1).unwrap();
    /// assert_eq!(summed.shape(), &[2, 4]);
    /// ```
    pub fn sum_axis(&self, axis: usize) -> Result<Tensor<T>>
    where
        T: Zero + std::ops::Add<Output = T>,
    {
        self.validate_axis(axis)?;

        let result = self
            .inner
            .sum_axis(Axis(axis));

        Ok(Tensor::new(result))
    }

    /// Returns the maximum value across all elements.
    pub fn max(&self) -> Result<T>
    where
        T: PartialOrd,
    {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        Ok(self
            .inner
            .iter()
            .cloned()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap())
    }

    /// Returns the maximum along the specified axis.
    pub fn max_axis(&self, axis: usize) -> Result<Tensor<T>>
    where
        T: PartialOrd,
    {
        self.validate_axis(axis)?;

        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        let result = self.inner.map_axis(Axis(axis), |slice| {
            slice
                .iter()
                .cloned()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        });

        Ok(Tensor::new(result))
    }

    /// Returns the minimum value across all elements.
    pub fn min(&self) -> Result<T>
    where
        T: PartialOrd,
    {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        Ok(self
            .inner
            .iter()
            .cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap())
    }

    /// Returns the index of the maximum value.
    pub fn argmax(&self) -> Result<usize>
    where
        T: PartialOrd,
    {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        let (idx, _) = self
            .inner
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        Ok(idx)
    }

    /// Returns the index of the minimum value.
    pub fn argmin(&self) -> Result<usize>
    where
        T: PartialOrd,
    {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        let (idx, _) = self
            .inner
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        Ok(idx)
    }

    fn validate_axis(&self, axis: usize) -> Result<()> {
        if axis >= self.rank() {
            return Err(Error::AxisOutOfBounds {
                axis,
                rank: self.rank(),
            });
        }
        Ok(())
    }
}

impl Tensor<f64> {
    /// Returns the mean of all elements for f64 tensors.
    pub fn mean(&self) -> Result<f64> {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        let sum = self.sum()?;
        let count = self.size() as f64;

        Ok(sum / count)
    }
}

impl Tensor<f32> {
    /// Returns the mean of all elements for f32 tensors.
    pub fn mean(&self) -> Result<f32> {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        let sum = self.sum()?;
        let count = self.size() as f32;

        Ok(sum / count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        let arr = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 3]),
            vec![1, 2, 3, 4, 5, 6],
        )
        .unwrap();
        let t: Tensor<i32> = Tensor::new(arr);
        assert_eq!(t.sum().unwrap(), 21);
    }

    #[test]
    fn test_sum_axis() {
        let arr = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 3]),
            vec![1, 2, 3, 4, 5, 6],
        )
        .unwrap();
        let t: Tensor<i32> = Tensor::new(arr);
        let summed = t.sum_axis(0).unwrap();
        assert_eq!(summed.shape(), &[3]);
    }

    #[test]
    fn test_sum_axis_out_of_bounds() {
        let t: Tensor<i32> = Tensor::zeros(&[2, 3]);
        let result = t.sum_axis(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_argmax() {
        let arr = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 3]),
            vec![1.0, 5.0, 3.0, 4.0, 2.0, 6.0],
        )
        .unwrap();
        let t: Tensor<f32> = Tensor::new(arr);
        assert_eq!(t.argmax().unwrap(), 5);
    }

    #[test]
    fn test_argmin() {
        let arr = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 3]),
            vec![1.0, 5.0, 3.0, 4.0, 2.0, 6.0],
        )
        .unwrap();
        let t: Tensor<f32> = Tensor::new(arr);
        assert_eq!(t.argmin().unwrap(), 0);
    }
}
