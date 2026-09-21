use ndarray::Array2;
use num_traits::Float;
use tensor_core::Tensor;

use crate::error::{LinAlgError, LinAlgResult};
use crate::util::to_array2;

/// A (thin) singular value decomposition: `A == U @ diag(s) @ V^T`, with
/// `U` (`m x k`) and `V` (`n x k`) having orthonormal columns, `k =
/// min(m, n)`, and `s` sorted descending.
#[derive(Debug, Clone)]
pub struct SvdDecomposition<T> {
    pub u: Tensor<T>,
    pub s: Tensor<T>,
    pub v: Tensor<T>,
}

/// One-sided Jacobi SVD (the Hestenes method): iteratively apply Givens
/// rotations to pairs of columns of `A` until they're pairwise
/// orthogonal, at which point their norms are the singular values and
/// their normalized directions are `U`; the accumulated rotations are
/// `V`. Straightforward to implement correctly and numerically robust,
/// at the cost of being `O(sweeps * m * n^2)` rather than
/// bidiagonalization-based.
pub fn svd<T: Float>(
    a: &Tensor<T>,
    max_sweeps: usize,
    tol: T,
) -> LinAlgResult<SvdDecomposition<T>> {
    let mat = to_array2(a, "svd")?;
    let (m, n) = mat.dim();

    if m < n {
        let transposed = Tensor::from_owned(mat.t().to_owned());
        let SvdDecomposition { u, s, v } = svd(&transposed, max_sweeps, tol)?;
        return Ok(SvdDecomposition { u: v, s, v: u });
    }

    let mut w = mat;
    let mut v = Array2::<T>::eye(n);
    let mut converged = n <= 1;

    for _ in 0..max_sweeps {
        let mut max_ratio = T::zero();
        for p in 0..n {
            for q in (p + 1)..n {
                let mut alpha = T::zero();
                let mut beta = T::zero();
                let mut gamma = T::zero();
                for i in 0..m {
                    let wip = w[[i, p]];
                    let wiq = w[[i, q]];
                    alpha = alpha + wip * wip;
                    beta = beta + wiq * wiq;
                    gamma = gamma + wip * wiq;
                }
                let denom = (alpha * beta).sqrt();
                if denom <= T::epsilon() {
                    continue;
                }
                let ratio = gamma.abs() / denom;
                if ratio > max_ratio {
                    max_ratio = ratio;
                }
                if ratio <= tol {
                    continue;
                }

                let zeta = (beta - alpha) / (gamma + gamma);
                let t = if zeta >= T::zero() {
                    T::one() / (zeta + (zeta * zeta + T::one()).sqrt())
                } else {
                    -T::one() / (-zeta + (zeta * zeta + T::one()).sqrt())
                };
                let c = T::one() / (t * t + T::one()).sqrt();
                let s = c * t;

                for i in 0..m {
                    let wip = w[[i, p]];
                    let wiq = w[[i, q]];
                    w[[i, p]] = c * wip - s * wiq;
                    w[[i, q]] = s * wip + c * wiq;
                }
                for i in 0..n {
                    let vip = v[[i, p]];
                    let viq = v[[i, q]];
                    v[[i, p]] = c * vip - s * viq;
                    v[[i, q]] = s * vip + c * viq;
                }
            }
        }
        if max_ratio <= tol {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(LinAlgError::ConvergenceFailure {
            op: "svd",
            max_iter: max_sweeps,
        });
    }

    let sigmas: Vec<T> = (0..n)
        .map(|j| {
            (0..m)
                .fold(T::zero(), |acc, i| acc + w[[i, j]] * w[[i, j]])
                .sqrt()
        })
        .collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| sigmas[j].partial_cmp(&sigmas[i]).unwrap());

    let mut u = Array2::<T>::zeros((m, n));
    let mut v_out = Array2::<T>::zeros((n, n));
    let mut s_out = vec![T::zero(); n];
    for (col, &src) in order.iter().enumerate() {
        let sigma = sigmas[src];
        s_out[col] = sigma;
        for i in 0..m {
            u[[i, col]] = if sigma > T::epsilon() {
                w[[i, src]] / sigma
            } else {
                T::zero()
            };
        }
        for i in 0..n {
            v_out[[i, col]] = v[[i, src]];
        }
    }

    Ok(SvdDecomposition {
        u: Tensor::from_owned(u),
        s: Tensor::from_vec(&[n], s_out).map_err(LinAlgError::Tensor)?,
        v: Tensor::from_owned(v_out),
    })
}
