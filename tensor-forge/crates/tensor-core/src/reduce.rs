use ndarray::{ArrayD, Axis, Dimension, IxDyn};
use num_traits::{Float, FromPrimitive, One, Zero};

use crate::error::{TensorError, TensorResult};
use crate::tensor::Tensor;

fn check_axis(axis: usize, ndim: usize) -> TensorResult<()> {
    if axis >= ndim {
        Err(TensorError::AxisOutOfBounds { axis, ndim })
    } else {
        Ok(())
    }
}

impl<T: Clone + Zero + std::ops::Add<Output = T>> Tensor<T> {
    /// The sum of all elements.
    pub fn sum(&self) -> T {
        self.data.sum()
    }

    /// The sum along `axis`, collapsing it (the result has rank
    /// `self.ndim() - 1`).
    pub fn sum_axis(&self, axis: usize) -> TensorResult<Tensor<T>> {
        check_axis(axis, self.ndim())?;
        Ok(Tensor::from_array(self.data.sum_axis(Axis(axis))))
    }
}

impl<T: Clone + One + std::ops::Mul<Output = T>> Tensor<T> {
    /// The product of all elements.
    pub fn product(&self) -> T {
        self.data.product()
    }
}

impl<T: Float + FromPrimitive> Tensor<T> {
    /// The arithmetic mean of all elements. `None` for an empty tensor.
    pub fn mean(&self) -> Option<T> {
        self.data.mean()
    }

    /// The arithmetic mean along `axis`.
    pub fn mean_axis(&self, axis: usize) -> TensorResult<Tensor<T>> {
        check_axis(axis, self.ndim())?;
        self.data
            .mean_axis(Axis(axis))
            .map(Tensor::from_array)
            .ok_or(TensorError::EmptyTensor)
    }

    /// The population/sample variance of all elements, with `ddof` degrees
    /// of freedom subtracted from the element count (`ddof = 0` for the
    /// population variance, `ddof = 1` for the unbiased sample variance).
    pub fn var(&self, ddof: usize) -> TensorResult<T> {
        let n = self.len();
        if n <= ddof {
            return Err(TensorError::EmptyTensor);
        }
        let mean = self.mean().ok_or(TensorError::EmptyTensor)?;
        let sq_dev_sum = self.data.iter().fold(T::zero(), |acc, &x| {
            let d = x - mean;
            acc + d * d
        });
        Ok(sq_dev_sum / T::from(n - ddof).expect("n - ddof fits in T"))
    }

    /// The standard deviation of all elements (`sqrt(var(ddof))`).
    pub fn std(&self, ddof: usize) -> TensorResult<T> {
        self.var(ddof).map(|v| v.sqrt())
    }

    /// Variance along `axis`.
    pub fn var_axis(&self, axis: usize, ddof: usize) -> TensorResult<Tensor<T>> {
        check_axis(axis, self.ndim())?;
        let n = self.shape()[axis];
        if n <= ddof {
            return Err(TensorError::EmptyTensor);
        }
        let mean = self
            .data
            .mean_axis(Axis(axis))
            .ok_or(TensorError::EmptyTensor)?;
        let denom = T::from(n - ddof).expect("n - ddof fits in T");
        let ndim = self.ndim();
        let out = ArrayD::from_shape_fn(mean.raw_dim(), |idx: IxDyn| {
            let reduced = idx.slice();
            let mut full_idx = vec![0usize; ndim];
            for (ri, slot) in full_idx
                .iter_mut()
                .enumerate()
                .filter_map(|(ax, slot)| (ax != axis).then_some(slot))
                .enumerate()
            {
                *slot = reduced[ri];
            }
            let m = mean[idx.clone()];
            let mut sum = T::zero();
            for i in 0..n {
                full_idx[axis] = i;
                let x = self.data[IxDyn(&full_idx)];
                let d = x - m;
                sum = sum + d * d;
            }
            sum / denom
        });
        Ok(Tensor::from_array(out))
    }

    /// Standard deviation along `axis`.
    pub fn std_axis(&self, axis: usize, ddof: usize) -> TensorResult<Tensor<T>> {
        self.var_axis(axis, ddof).map(|t| t.mapv(|x| x.sqrt()))
    }
}

impl<T: PartialOrd + Clone> Tensor<T> {
    /// The maximum element, compared with `partial_cmp` (so e.g. `NaN`
    /// comparisons behave as IEEE-754 specifies: any comparison involving
    /// `NaN` is `false`, so a `NaN` element will not "win" a max/min scan
    /// unless every element is `NaN`).
    pub fn max(&self) -> TensorResult<T> {
        self.data
            .iter()
            .fold(None, |acc: Option<&T>, x| match acc {
                None => Some(x),
                Some(m) if x.partial_cmp(m) == Some(std::cmp::Ordering::Greater) => Some(x),
                Some(m) => Some(m),
            })
            .cloned()
            .ok_or(TensorError::EmptyTensor)
    }

    /// The minimum element. See [`Tensor::max`] for comparison semantics.
    pub fn min(&self) -> TensorResult<T> {
        self.data
            .iter()
            .fold(None, |acc: Option<&T>, x| match acc {
                None => Some(x),
                Some(m) if x.partial_cmp(m) == Some(std::cmp::Ordering::Less) => Some(x),
                Some(m) => Some(m),
            })
            .cloned()
            .ok_or(TensorError::EmptyTensor)
    }

    /// The flat (row-major) index of the maximum element.
    pub fn argmax(&self) -> TensorResult<usize> {
        self.data
            .iter()
            .enumerate()
            .fold(None, |acc: Option<(usize, &T)>, (i, x)| match acc {
                None => Some((i, x)),
                Some((_, m)) if x.partial_cmp(m) == Some(std::cmp::Ordering::Greater) => {
                    Some((i, x))
                }
                Some(prev) => Some(prev),
            })
            .map(|(i, _)| i)
            .ok_or(TensorError::EmptyTensor)
    }

    /// The flat (row-major) index of the minimum element.
    pub fn argmin(&self) -> TensorResult<usize> {
        self.data
            .iter()
            .enumerate()
            .fold(None, |acc: Option<(usize, &T)>, (i, x)| match acc {
                None => Some((i, x)),
                Some((_, m)) if x.partial_cmp(m) == Some(std::cmp::Ordering::Less) => Some((i, x)),
                Some(prev) => Some(prev),
            })
            .map(|(i, _)| i)
            .ok_or(TensorError::EmptyTensor)
    }

    /// The maximum along `axis`, collapsing it.
    pub fn max_axis(&self, axis: usize) -> TensorResult<Tensor<T>> {
        check_axis(axis, self.ndim())?;
        if self.shape()[axis] == 0 {
            return Err(TensorError::EmptyTensor);
        }
        Ok(Tensor::from_array(self.data.map_axis(Axis(axis), |view| {
            view.iter()
                .fold(view[0].clone(), |m, x| if *x > m { x.clone() } else { m })
        })))
    }

    /// The minimum along `axis`, collapsing it.
    pub fn min_axis(&self, axis: usize) -> TensorResult<Tensor<T>> {
        check_axis(axis, self.ndim())?;
        if self.shape()[axis] == 0 {
            return Err(TensorError::EmptyTensor);
        }
        Ok(Tensor::from_array(self.data.map_axis(Axis(axis), |view| {
            view.iter()
                .fold(view[0].clone(), |m, x| if *x < m { x.clone() } else { m })
        })))
    }

    /// The index (along `axis`) of the maximum element, for every
    /// remaining index combination -- i.e. `argmax` broadcast over `axis`.
    pub fn argmax_axis(&self, axis: usize) -> TensorResult<Tensor<usize>> {
        check_axis(axis, self.ndim())?;
        if self.shape()[axis] == 0 {
            return Err(TensorError::EmptyTensor);
        }
        Ok(Tensor::from_array(self.data.map_axis(Axis(axis), |view| {
            view.iter()
                .enumerate()
                .fold((0usize, view[0].clone()), |(bi, bv), (i, x)| {
                    if *x > bv { (i, x.clone()) } else { (bi, bv) }
                })
                .0
        })))
    }

    /// The index (along `axis`) of the minimum element.
    pub fn argmin_axis(&self, axis: usize) -> TensorResult<Tensor<usize>> {
        check_axis(axis, self.ndim())?;
        if self.shape()[axis] == 0 {
            return Err(TensorError::EmptyTensor);
        }
        Ok(Tensor::from_array(self.data.map_axis(Axis(axis), |view| {
            view.iter()
                .enumerate()
                .fold((0usize, view[0].clone()), |(bi, bv), (i, x)| {
                    if *x < bv { (i, x.clone()) } else { (bi, bv) }
                })
                .0
        })))
    }
}
