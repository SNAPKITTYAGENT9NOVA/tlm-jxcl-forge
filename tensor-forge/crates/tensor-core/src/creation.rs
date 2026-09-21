use ndarray::{Array, Array1, ArrayD, Dimension, IxDyn, ShapeBuilder};
use num_traits::{Float, One, Zero};

use crate::error::{TensorError, TensorResult};
use crate::tensor::Tensor;

impl<T: Clone + Zero> Tensor<T> {
    /// A tensor of the given shape, filled with zeros.
    pub fn zeros(shape: &[usize]) -> Self {
        Tensor::from_array(ArrayD::zeros(IxDyn(shape)))
    }
}

impl<T: Clone + One> Tensor<T> {
    /// A tensor of the given shape, filled with ones.
    pub fn ones(shape: &[usize]) -> Self {
        Tensor::from_array(ArrayD::ones(IxDyn(shape)))
    }
}

impl<T: Clone> Tensor<T> {
    /// A tensor of the given shape, filled with `value`.
    pub fn full(shape: &[usize], value: T) -> Self {
        Tensor::from_array(ArrayD::from_elem(IxDyn(shape), value))
    }

    /// Build a tensor by evaluating `f` at every index of `shape`.
    ///
    /// `f` receives the multi-index (one `usize` per axis) of the element
    /// being produced, outermost axis first.
    pub fn from_shape_fn(shape: &[usize], mut f: impl FnMut(&[usize]) -> T) -> Self {
        Tensor::from_array(ArrayD::from_shape_fn(IxDyn(shape), |idx| f(idx.slice())))
    }

    /// Build a tensor from a flat, row-major (C order) `Vec` and a shape.
    ///
    /// Fails with [`TensorError::DataLengthMismatch`] if `data.len()`
    /// does not equal the product of `shape`.
    pub fn from_vec(shape: &[usize], data: Vec<T>) -> TensorResult<Self> {
        let shape_len: usize = shape.iter().product();
        let data_len = data.len();
        Array::from_shape_vec(IxDyn(shape), data)
            .map(Tensor::from_array)
            .map_err(|_| TensorError::DataLengthMismatch {
                shape: shape.to_vec(),
                shape_len,
                data_len,
            })
    }

    /// Build a tensor from a flat, column-major (Fortran order) `Vec` and
    /// a shape.
    pub fn from_vec_f(shape: &[usize], data: Vec<T>) -> TensorResult<Self> {
        let shape_len: usize = shape.iter().product();
        let data_len = data.len();
        Array::from_shape_vec(IxDyn(shape).f(), data)
            .map(Tensor::from_array)
            .map_err(|_| TensorError::DataLengthMismatch {
                shape: shape.to_vec(),
                shape_len,
                data_len,
            })
    }
}

impl<T: Clone + Zero + One> Tensor<T> {
    /// The `n x n` identity matrix, as a rank-2 tensor.
    pub fn eye(n: usize) -> Self {
        Tensor::from_array(Array::eye(n).into_dyn())
    }
}

impl<T: Float> Tensor<T> {
    /// A 1-D tensor `[start, start + step, start + 2*step, ...)`, stopping
    /// strictly before `stop`. Mirrors NumPy's `arange`.
    pub fn arange(start: T, stop: T, step: T) -> TensorResult<Self> {
        if step.is_zero() {
            return Err(TensorError::InvalidStep);
        }
        let mut values = Vec::new();
        let mut current = start;
        if step > T::zero() {
            while current < stop {
                values.push(current);
                current = current + step;
            }
        } else {
            while current > stop {
                values.push(current);
                current = current + step;
            }
        }
        Ok(Tensor::from_array(Array1::from_vec(values).into_dyn()))
    }

    /// A 1-D tensor of `num` evenly spaced values from `start` to `stop`
    /// (inclusive of both endpoints).
    pub fn linspace(start: T, stop: T, num: usize) -> Self {
        Tensor::from_array(Array1::linspace(start, stop, num).into_dyn())
    }
}

#[cfg(feature = "rand")]
mod random {
    use super::*;
    use ndarray_rand::RandomExt;
    use rand_distr::{Distribution, Normal, StandardUniform, Uniform, uniform::SampleUniform};

    impl<T> Tensor<T>
    where
        T: Clone + SampleUniform + PartialOrd,
    {
        /// A tensor of the given shape with elements drawn uniformly from
        /// `[low, high)`. Requires the `rand` feature.
        pub fn random_uniform(shape: &[usize], low: T, high: T) -> Self {
            let dist = Uniform::new(low, high).expect("low < high");
            Tensor::from_array(ArrayD::random(IxDyn(shape), dist))
        }
    }

    impl<T> Tensor<T>
    where
        StandardUniform: Distribution<T>,
    {
        /// A tensor of the given shape with elements drawn from the
        /// default distribution for `T` (uniform over `[0, 1)` for
        /// floats). Requires the `rand` feature.
        pub fn random(shape: &[usize]) -> Self {
            Tensor::from_array(ArrayD::random(IxDyn(shape), StandardUniform))
        }
    }

    impl<T: Float> Tensor<T>
    where
        rand_distr::StandardNormal: Distribution<T>,
    {
        /// A tensor of the given shape with elements drawn from
        /// `Normal(mean, std_dev)`. Requires the `rand` feature.
        pub fn random_normal(shape: &[usize], mean: T, std_dev: T) -> TensorResult<Self> {
            let dist = Normal::new(mean, std_dev)
                .map_err(|e| TensorError::InvalidPattern(e.to_string()))?;
            Ok(Tensor::from_array(ArrayD::random(IxDyn(shape), dist)))
        }
    }
}
