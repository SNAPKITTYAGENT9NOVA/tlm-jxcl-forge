use ndarray::{ArrayD, Zip};
use num_traits::Float;
use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::error::{TensorError, TensorResult};
use crate::tensor::Tensor;

/// NumPy-style broadcasting: align two shapes from the trailing (rightmost)
/// axis, requiring each pair of aligned dimensions to be equal or for one
/// of them to be `1`. Returns the resulting broadcast shape, or `None` if
/// the shapes are incompatible.
pub fn broadcast_shape(a: &[usize], b: &[usize]) -> Option<Vec<usize>> {
    let mut result = Vec::with_capacity(a.len().max(b.len()));
    let mut ai = a.iter().rev();
    let mut bi = b.iter().rev();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => break,
            (Some(&x), None) | (None, Some(&x)) => result.push(x),
            (Some(&x), Some(&y)) => {
                if x == y {
                    result.push(x);
                } else if x == 1 {
                    result.push(y);
                } else if y == 1 {
                    result.push(x);
                } else {
                    return None;
                }
            }
        }
    }
    result.reverse();
    Some(result)
}

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $checked:ident, $op:tt) => {
        impl<T> $trait<&Tensor<T>> for &Tensor<T>
        where
            for<'a> &'a ArrayD<T>: $trait<&'a ArrayD<T>, Output = ArrayD<T>>,
        {
            type Output = Tensor<T>;

            /// Elementwise, broadcasting. Panics on shape mismatch (mirrors
            /// `ndarray`/NumPy); use the `checked_*` method for a `Result`.
            fn $method(self, rhs: &Tensor<T>) -> Tensor<T> {
                Tensor::from_array(&self.data $op &rhs.data)
            }
        }

        impl<T> $trait<Tensor<T>> for Tensor<T>
        where
            for<'a> &'a ArrayD<T>: $trait<&'a ArrayD<T>, Output = ArrayD<T>>,
        {
            type Output = Tensor<T>;

            fn $method(self, rhs: Tensor<T>) -> Tensor<T> {
                &self $op &rhs
            }
        }

        impl<T> Tensor<T>
        where
            for<'a> &'a ArrayD<T>: $trait<&'a ArrayD<T>, Output = ArrayD<T>>,
        {
            /// Elementwise, broadcasting. Returns
            /// [`TensorError::BroadcastError`] instead of panicking when
            /// the shapes cannot be broadcast together.
            pub fn $checked(&self, rhs: &Tensor<T>) -> TensorResult<Tensor<T>> {
                if broadcast_shape(self.shape(), rhs.shape()).is_none() {
                    return Err(TensorError::BroadcastError {
                        from: rhs.shape().to_vec(),
                        to: self.shape().to_vec(),
                    });
                }
                Ok(self $op rhs)
            }
        }
    };
}

impl_binop!(Add, add, checked_add, +);
impl_binop!(Sub, sub, checked_sub, -);
impl_binop!(Mul, mul, checked_mul, *);
impl_binop!(Div, div, checked_div, /);

impl<T> Neg for Tensor<T>
where
    ArrayD<T>: Neg<Output = ArrayD<T>>,
{
    type Output = Tensor<T>;

    fn neg(self) -> Tensor<T> {
        Tensor::from_array(-self.data)
    }
}

macro_rules! impl_scalar_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl<T> $trait<T> for Tensor<T>
        where
            T: Clone + $trait<T, Output = T> + ndarray::ScalarOperand,
        {
            type Output = Tensor<T>;

            fn $method(self, rhs: T) -> Tensor<T> {
                Tensor::from_array(self.data $op rhs)
            }
        }
    };
}

impl_scalar_op!(Add, add, +);
impl_scalar_op!(Sub, sub, -);
impl_scalar_op!(Mul, mul, *);
impl_scalar_op!(Div, div, /);

impl<T: Clone> Tensor<T> {
    /// Apply `f` to every element, producing a tensor of a possibly
    /// different element type (like `Iterator::map`).
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Tensor<U> {
        Tensor::from_array(self.data.map(f))
    }

    /// Apply `f` to every element in place.
    pub fn mapv_inplace(&mut self, f: impl FnMut(T) -> T) {
        self.data.mapv_inplace(f);
    }

    /// Apply `f` to every element, by value, producing a new tensor of the
    /// same shape (like `ndarray`'s `mapv`).
    pub fn mapv(&self, f: impl FnMut(T) -> T) -> Tensor<T> {
        Tensor::from_array(self.data.mapv(f))
    }
}

impl<T: Float> Tensor<T> {
    pub fn abs(&self) -> Tensor<T> {
        self.mapv(|x| x.abs())
    }
    pub fn sqrt(&self) -> Tensor<T> {
        self.mapv(|x| x.sqrt())
    }
    pub fn exp(&self) -> Tensor<T> {
        self.mapv(|x| x.exp())
    }
    pub fn ln(&self) -> Tensor<T> {
        self.mapv(|x| x.ln())
    }
    pub fn sin(&self) -> Tensor<T> {
        self.mapv(|x| x.sin())
    }
    pub fn cos(&self) -> Tensor<T> {
        self.mapv(|x| x.cos())
    }
    pub fn tan(&self) -> Tensor<T> {
        self.mapv(|x| x.tan())
    }
    pub fn tanh(&self) -> Tensor<T> {
        self.mapv(|x| x.tanh())
    }
    pub fn recip(&self) -> Tensor<T> {
        self.mapv(|x| x.recip())
    }
    pub fn powi(&self, n: i32) -> Tensor<T> {
        self.mapv(|x| x.powi(n))
    }
    pub fn powf(&self, n: T) -> Tensor<T> {
        self.mapv(|x| x.powf(n))
    }
    pub fn clamp(&self, min: T, max: T) -> Tensor<T> {
        self.mapv(|x| x.max(min).min(max))
    }

    /// Rectified linear unit: `max(x, 0)`, elementwise.
    pub fn relu(&self) -> Tensor<T> {
        self.mapv(|x| x.max(T::zero()))
    }

    /// Elementwise fused multiply-add: `self * b + c`. Requires the three
    /// tensors to already share a shape (broadcasting a 3-way fused op
    /// unambiguously would need a NumPy-style `out=` shape decision this
    /// API does not make for you -- broadcast the operands explicitly
    /// first if needed).
    pub fn mul_add(&self, b: &Tensor<T>, c: &Tensor<T>) -> TensorResult<Tensor<T>> {
        if self.shape() != b.shape() || self.shape() != c.shape() {
            return Err(TensorError::ShapeMismatch {
                expected: self.shape().to_vec(),
                actual: if self.shape() != b.shape() {
                    b.shape().to_vec()
                } else {
                    c.shape().to_vec()
                },
            });
        }
        let mut out = ArrayD::zeros(self.data.raw_dim());
        Zip::from(&mut out)
            .and(&self.data)
            .and(&b.data)
            .and(&c.data)
            .for_each(|o, &x, &y, &z| *o = x.mul_add(y, z));
        Ok(Tensor::from_array(out))
    }
}

impl<T: PartialEq> Tensor<T> {
    /// Elementwise equality, producing a boolean tensor of the same shape.
    /// Panics if shapes differ (no broadcasting; compare `.shape()` first
    /// if that's a possibility).
    pub fn eq_elementwise(&self, other: &Tensor<T>) -> Tensor<bool> {
        assert_eq!(
            self.shape(),
            other.shape(),
            "eq_elementwise: shape mismatch"
        );
        Tensor::from_array(
            Zip::from(&self.data)
                .and(&other.data)
                .map_collect(|a, b| a == b),
        )
    }
}
