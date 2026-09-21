use ndarray::Array2;
use num_traits::Float;
use tensor_core::Tensor;

use crate::error::LinAlgResult;
use crate::util::to_array2;

/// A QR decomposition: `A == Q @ R`, with `Q` (`m x m`) orthogonal and `R`
/// (`m x n`) upper-triangular.
#[derive(Debug, Clone)]
pub struct QrDecomposition<T> {
    pub q: Tensor<T>,
    pub r: Tensor<T>,
}

/// Householder QR decomposition, the standard numerically stable way to
/// compute one: `A` is reduced to upper-triangular `R` by a sequence of
/// Householder reflections, whose product (accumulated on the right of an
/// identity) gives `Q`.
pub fn qr<T: Float>(a: &Tensor<T>) -> LinAlgResult<QrDecomposition<T>> {
    let mut r = to_array2(a, "qr")?;
    let (m, n) = r.dim();
    let mut q = Array2::<T>::eye(m);
    let steps = m.min(n);

    for k in 0..steps {
        let mut norm_sq = T::zero();
        for i in k..m {
            norm_sq = norm_sq + r[[i, k]] * r[[i, k]];
        }
        let norm = norm_sq.sqrt();
        if norm <= T::epsilon() {
            continue;
        }
        let alpha = if r[[k, k]] > T::zero() { -norm } else { norm };

        let mut v = vec![T::zero(); m];
        for (i, slot) in v.iter_mut().enumerate().take(m).skip(k) {
            *slot = r[[i, k]];
        }
        v[k] = v[k] - alpha;
        let v_norm_sq: T = v[k..m].iter().fold(T::zero(), |acc, &x| acc + x * x);
        if v_norm_sq <= T::epsilon() {
            continue;
        }

        // Apply the reflection to R (from the left).
        for j in k..n {
            let dot = (k..m).fold(T::zero(), |acc, i| acc + v[i] * r[[i, j]]);
            let coeff = (dot + dot) / v_norm_sq;
            for i in k..m {
                r[[i, j]] = r[[i, j]] - coeff * v[i];
            }
        }
        // Accumulate the same reflection into Q (from the right: Q := Q H).
        for i in 0..m {
            let dot = (k..m).fold(T::zero(), |acc, l| acc + q[[i, l]] * v[l]);
            let coeff = (dot + dot) / v_norm_sq;
            for l in k..m {
                q[[i, l]] = q[[i, l]] - coeff * v[l];
            }
        }
    }

    Ok(QrDecomposition {
        q: Tensor::from_owned(q),
        r: Tensor::from_owned(r),
    })
}
