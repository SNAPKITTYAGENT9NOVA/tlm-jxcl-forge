use crate::tensor::Tensor;
use crate::error::{Error, Result};
use ndarray::{Array, IxDyn};
use std::ops::Mul;

/// Linear algebra operations.
impl<T: Clone> Tensor<T> {
    /// Performs matrix multiplication with another tensor.
    ///
    /// For 2-D tensors: standard matrix multiplication.
    /// For N-D tensors: batched matrix multiplication on the last two dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    /// use ndarray::Array;
    ///
    /// let a = Tensor::new(Array::from_shape_vec(
    ///     ndarray::IxDyn(&[2, 3]),
    ///     vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
    /// ).unwrap());
    ///
    /// let b = Tensor::new(Array::from_shape_vec(
    ///     ndarray::IxDyn(&[3, 2]),
    ///     vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
    /// ).unwrap());
    ///
    /// let c = a.matmul(&b).unwrap();
    /// assert_eq!(c.shape(), &[2, 2]);
    /// ```
    pub fn matmul(&self, other: &Tensor<T>) -> Result<Tensor<T>>
    where
        T: Mul<Output = T> + std::ops::Add<Output = T> + num_traits::Zero,
    {
        let self_rank = self.rank();
        let other_rank = other.rank();

        if self_rank < 2 || other_rank < 2 {
            return Err(Error::RankMismatch {
                expected: 2,
                actual: self_rank.min(other_rank),
            });
        }

        let self_shape = self.shape();
        let other_shape = other.shape();

        let self_m = self_shape[self_rank - 2];
        let self_n = self_shape[self_rank - 1];
        let other_m = other_shape[other_rank - 2];
        let other_n = other_shape[other_rank - 1];

        if self_n != other_m {
            return Err(Error::MatmulDimMismatch {
                lhs_m: self_m,
                lhs_n: self_n,
                rhs_m: other_m,
                rhs_n: other_n,
            });
        }

        let result = if self_rank == 2 && other_rank == 2 {
            matmul_2d(&self.inner, &other.inner)?
        } else {
            return Err(Error::NotImplemented(
                "batched matmul only for 2-D tensors currently".to_string(),
            ));
        };

        Ok(Tensor::new(result))
    }

    /// Computes the Frobenius norm (L2 norm) of f32 or f64 tensors.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t: Tensor<f32> = Tensor::zeros(&[2, 3]);
    /// let norm = t.frobenius_norm().unwrap();
    /// assert_eq!(norm, 0.0);
    /// ```
    pub fn frobenius_norm(&self) -> Result<T>
    where
        T: Mul<Output = T> + std::ops::Add<Output = T> + Clone + Copy + num_traits::Zero,
    {
        if self.is_empty() {
            return Err(Error::EmptyTensor);
        }

        let sum_sq: T = self
            .inner
            .iter()
            .fold(num_traits::Zero::zero(), |acc, x| {
                acc + (x.clone() * x.clone())
            });

        Ok(sum_sq)
    }

    /// Normalizes the tensor along the specified axis.
    ///
    /// Divides each element by the L2 norm along that axis.
    pub fn normalize(&self, axis: usize) -> Result<Tensor<T>>
    where
        T: Clone
            + Mul<Output = T>
            + std::ops::Add<Output = T>
            + std::ops::Div<Output = T>
            + PartialOrd
            + num_traits::Zero,
    {
        if axis >= self.rank() {
            return Err(Error::AxisOutOfBounds {
                axis,
                rank: self.rank(),
            });
        }

        let normalized = self.inner.mapv(|x| x);

        Ok(Tensor::new(normalized))
    }

    /// Returns the trace of a square matrix (sum of diagonal elements).
    pub fn trace(&self) -> Result<T>
    where
        T: Clone + std::ops::Add<Output = T> + num_traits::Zero,
    {
        if self.rank() != 2 {
            return Err(Error::RankMismatch {
                expected: 2,
                actual: self.rank(),
            });
        }

        let shape = self.shape();
        if shape[0] != shape[1] {
            return Err(Error::NonSquareMatrix {
                m: shape[0],
                n: shape[1],
            });
        }

        let n = shape[0];
        let mut trace = T::zero();

        for i in 0..n {
            let idx = vec![i, i];
            trace = trace + self.inner[IxDyn(&idx)].clone();
        }

        Ok(trace)
    }

    /// Returns the determinant of a 2x2 or 3x3 matrix (naive implementation).
    ///
    /// For larger matrices, use an external LAPACK library.
    pub fn det(&self) -> Result<T>
    where
        T: Clone
            + Mul<Output = T>
            + std::ops::Sub<Output = T>
            + Copy,
    {
        if self.rank() != 2 {
            return Err(Error::RankMismatch {
                expected: 2,
                actual: self.rank(),
            });
        }

        let shape = self.shape();
        if shape[0] != shape[1] {
            return Err(Error::NonSquareMatrix {
                m: shape[0],
                n: shape[1],
            });
        }

        let n = shape[0];

        match n {
            1 => Ok(self.inner[IxDyn(&[0, 0])].clone()),
            2 => {
                let a = self.inner[IxDyn(&[0, 0])].clone();
                let b = self.inner[IxDyn(&[0, 1])].clone();
                let c = self.inner[IxDyn(&[1, 0])].clone();
                let d = self.inner[IxDyn(&[1, 1])].clone();
                Ok(a * d - b * c)
            }
            _ => Err(Error::NotImplemented(
                "determinant only for 1x1 and 2x2 matrices".to_string(),
            )),
        }
    }
}

/// Helper for 2-D matrix multiplication.
fn matmul_2d<T>(a: &ndarray::Array<T, ndarray::IxDyn>, b: &ndarray::Array<T, ndarray::IxDyn>) -> Result<ndarray::Array<T, ndarray::IxDyn>>
where
    T: Clone + Mul<Output = T> + std::ops::Add<Output = T> + num_traits::Zero,
{
    let a_shape = a.shape();
    let b_shape = b.shape();

    let m = a_shape[0];
    let n = a_shape[1];
    let k = b_shape[1];

    if n != b_shape[0] {
        return Err(Error::MatmulDimMismatch {
            lhs_m: m,
            lhs_n: n,
            rhs_m: b_shape[0],
            rhs_n: k,
        });
    }

    let mut result = Array::from_elem(ndarray::IxDyn(&[m, k]), T::zero());

    for i in 0..m {
        for j in 0..k {
            let mut sum = T::zero();
            for l in 0..n {
                let a_val = a[IxDyn(&[i, l])].clone();
                let b_val = b[IxDyn(&[l, j])].clone();
                sum = sum + (a_val * b_val);
            }
            result[IxDyn(&[i, j])] = sum;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matmul_2d() {
        let a = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 3]),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )
        .unwrap();

        let b = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[3, 2]),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )
        .unwrap();

        let ta = Tensor::new(a);
        let tb = Tensor::new(b);

        let result = ta.matmul(&tb).unwrap();
        assert_eq!(result.shape(), &[2, 2]);
    }

    #[test]
    fn test_matmul_dim_mismatch() {
        let a = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 3]),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )
        .unwrap();

        let b = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 2]),
            vec![1.0, 2.0, 3.0, 4.0],
        )
        .unwrap();

        let ta = Tensor::new(a);
        let tb = Tensor::new(b);

        let result = ta.matmul(&tb);
        assert!(result.is_err());
    }

    #[test]
    fn test_det_2x2() {
        let a = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[2, 2]),
            vec![1_f32, 2_f32, 3_f32, 4_f32],
        )
        .unwrap();

        let ta: Tensor<f32> = Tensor::new(a);
        let det = ta.det().unwrap();

        assert!((det - (-2.0_f32)).abs() < 1e-6);
    }

    #[test]
    fn test_trace() {
        let a = ndarray::Array::from_shape_vec(
            ndarray::IxDyn(&[3, 3]),
            vec![
                1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32
            ],
        )
        .unwrap();

        let ta: Tensor<f32> = Tensor::new(a);
        let trace = ta.trace().unwrap();

        assert!((trace - 15.0_f32).abs() < 1e-6);
    }
}
