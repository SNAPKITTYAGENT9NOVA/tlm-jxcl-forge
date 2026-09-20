use crate::tensor::Tensor;
use crate::error::{Error, Result};
use ndarray::Zip;
use std::ops::{Add, Sub, Mul, Div};

/// Broadcasting-aware tensor operations.
///
/// All arithmetic operations automatically broadcast compatible shapes.
/// Broadcasting rules follow NumPy conventions:
/// - Dimensions of size 1 are stretched to match the other tensor
/// - Missing dimensions are treated as size 1
/// - Incompatible dimensions cause an error
impl<T> Tensor<T>
where
    T: Clone + Add<Output = T>,
{
    /// Adds two tensors element-wise with automatic broadcasting.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let a = Tensor::zeros(&[2, 3]);
    /// let b = Tensor::zeros(&[1, 3]);
    /// let c = a.add(&b).unwrap();
    /// assert_eq!(c.shape(), &[2, 3]);
    /// ```
    pub fn add(&self, other: &Tensor<T>) -> Result<Tensor<T>> {
        broadcast_binary_op(self, other, |x, y| x.clone() + y.clone())
    }
}

impl<T> Tensor<T>
where
    T: Clone + Sub<Output = T>,
{
    /// Subtracts two tensors element-wise with automatic broadcasting.
    pub fn sub(&self, other: &Tensor<T>) -> Result<Tensor<T>> {
        broadcast_binary_op(self, other, |x, y| x.clone() - y.clone())
    }
}

impl<T> Tensor<T>
where
    T: Clone + Mul<Output = T>,
{
    /// Multiplies two tensors element-wise with automatic broadcasting.
    pub fn mul(&self, other: &Tensor<T>) -> Result<Tensor<T>> {
        broadcast_binary_op(self, other, |x, y| x.clone() * y.clone())
    }
}

impl<T> Tensor<T>
where
    T: Clone + Div<Output = T>,
{
    /// Divides two tensors element-wise with automatic broadcasting.
    pub fn div(&self, other: &Tensor<T>) -> Result<Tensor<T>> {
        broadcast_binary_op(self, other, |x, y| x.clone() / y.clone())
    }
}

impl<T> Tensor<T>
where
    T: Clone + Mul<Output = T> + Default,
{
    /// Scales a tensor by a scalar.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_tensor::Tensor;
    ///
    /// let t = Tensor::zeros(&[2, 3]);
    /// let scaled = t.scale(&2.0).unwrap();
    /// assert_eq!(scaled.shape(), &[2, 3]);
    /// ```
    pub fn scale(&self, scalar: &T) -> Result<Tensor<T>> {
        let scaled = self.inner.mapv(|x| x * scalar.clone());
        Ok(Tensor::new(scaled))
    }
}

impl<T> Tensor<T>
where
    T: Clone + num_traits::One + Default,
{
    /// Applies the map function element-wise.
    pub fn map<F>(&self, f: F) -> Tensor<T>
    where
        F: Fn(&T) -> T,
    {
        let mapped = self.inner.mapv(|x| f(&x));
        Tensor::new(mapped)
    }
}

/// Checks if two shapes are broadcastable according to NumPy rules.
fn is_broadcastable(shape1: &[usize], shape2: &[usize]) -> bool {
    let len1 = shape1.len();
    let len2 = shape2.len();
    let max_len = len1.max(len2);

    let padded1 = {
        let mut v = vec![1; max_len - len1];
        v.extend_from_slice(shape1);
        v
    };

    let padded2 = {
        let mut v = vec![1; max_len - len2];
        v.extend_from_slice(shape2);
        v
    };

    padded1
        .iter()
        .zip(padded2.iter())
        .all(|(d1, d2)| d1 == d2 || *d1 == 1 || *d2 == 1)
}

/// Computes the broadcast output shape for two compatible shapes.
fn broadcast_shape(shape1: &[usize], shape2: &[usize]) -> Result<Vec<usize>> {
    if !is_broadcastable(shape1, shape2) {
        return Err(Error::BroadcastIncompatible {
            lhs: shape1.to_vec(),
            rhs: shape2.to_vec(),
        });
    }

    let len1 = shape1.len();
    let len2 = shape2.len();
    let max_len = len1.max(len2);

    let padded1 = {
        let mut v = vec![1; max_len - len1];
        v.extend_from_slice(shape1);
        v
    };

    let padded2 = {
        let mut v = vec![1; max_len - len2];
        v.extend_from_slice(shape2);
        v
    };

    Ok(padded1
        .iter()
        .zip(padded2.iter())
        .map(|(d1, d2)| d1.max(d2))
        .copied()
        .collect())
}

/// Broadcasts a tensor to a target shape.
fn broadcast<T: Clone>(tensor: &Tensor<T>, target_shape: &[usize]) -> Result<Tensor<T>> {
    let current_shape = tensor.shape();

    if current_shape == target_shape {
        return Ok(tensor.clone());
    }

    let mut broadcasted = tensor.inner.clone();

    for (_i, (curr, target)) in current_shape.iter().zip(target_shape.iter()).enumerate() {
        if curr != target && *curr != 1 {
            return Err(Error::BroadcastIncompatible {
                lhs: current_shape.to_vec(),
                rhs: target_shape.to_vec(),
            });
        }

        if *curr == 1 && *target > 1 {
            broadcasted = broadcasted.broadcast(target_shape).unwrap().to_owned();
            break;
        }
    }

    Ok(Tensor::new(broadcasted))
}

/// Helper for broadcasting binary operations.
fn broadcast_binary_op<T, F>(
    a: &Tensor<T>,
    b: &Tensor<T>,
    f: F,
) -> Result<Tensor<T>>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
{
    let out_shape = broadcast_shape(a.shape(), b.shape())?;
    let a_bc = broadcast(a, &out_shape)?;
    let b_bc = broadcast(b, &out_shape)?;

    let result = Zip::from(a_bc.inner.view())
        .and(b_bc.inner.view())
        .map_collect(|x, y| f(x, y));

    Ok(Tensor::new(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_broadcastable() {
        assert!(is_broadcastable(&[2, 3], &[3]));
        assert!(is_broadcastable(&[2, 3], &[1, 3]));
        assert!(is_broadcastable(&[2, 1, 3], &[1, 3]));
        assert!(!is_broadcastable(&[2, 3], &[4, 5]));
    }

    #[test]
    fn test_broadcast_shape() {
        let shape = broadcast_shape(&[2, 3], &[3]).unwrap();
        assert_eq!(shape, vec![2, 3]);

        let shape = broadcast_shape(&[2, 1], &[1, 3]).unwrap();
        assert_eq!(shape, vec![2, 3]);
    }

    #[test]
    fn test_add() {
        let a = Tensor::<f32>::zeros(&[2, 3]);
        let b = Tensor::<f32>::zeros(&[1, 3]);
        let c = a.add(&b).unwrap();
        assert_eq!(c.shape(), &[2, 3]);
    }

    #[test]
    fn test_broadcast_incompatible() {
        let a = Tensor::<f32>::zeros(&[2, 3]);
        let b = Tensor::<f32>::zeros(&[4, 5]);
        let result = a.add(&b);
        assert!(result.is_err());
    }
}
