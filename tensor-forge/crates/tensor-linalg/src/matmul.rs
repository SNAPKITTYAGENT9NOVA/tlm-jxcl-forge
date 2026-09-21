use std::collections::HashMap;

use ndarray::{Axis, Ix2, LinalgScalar};
use num_traits::Zero;
use tensor_core::Tensor;

use crate::error::{LinAlgError, LinAlgResult};
use crate::util::{to_array1, to_array2};

fn split_batch<'s>(
    shape: &'s [usize],
    op: &'static str,
) -> LinAlgResult<(&'s [usize], usize, usize)> {
    if shape.len() < 2 {
        return Err(LinAlgError::RankMismatch {
            op,
            expected: 2,
            actual: shape.len(),
        });
    }
    let n = shape.len();
    Ok((&shape[..n - 2], shape[n - 2], shape[n - 1]))
}

fn batched_matmul<T: LinalgScalar>(a: &Tensor<T>, b: &Tensor<T>) -> LinAlgResult<Tensor<T>> {
    let (a_batch, am, ak): (&[usize], usize, usize) = split_batch(a.shape(), "matmul")?;
    let (b_batch, bk, bn): (&[usize], usize, usize) = split_batch(b.shape(), "matmul")?;
    if a_batch != b_batch || ak != bk {
        return Err(LinAlgError::IncompatibleShapes {
            op: "matmul",
            lhs: a.shape().to_vec(),
            rhs: b.shape().to_vec(),
        });
    }
    let batch: usize = a_batch.iter().product();
    let a2 = a.reshape(&[batch, am, ak]).map_err(LinAlgError::Tensor)?;
    let b2 = b.reshape(&[batch, bk, bn]).map_err(LinAlgError::Tensor)?;

    let mut out_data = Vec::with_capacity(batch * am * bn);
    for i in 0..batch {
        let ai = a2
            .as_array()
            .index_axis(Axis(0), i)
            .into_dimensionality::<Ix2>()
            .unwrap();
        let bi = b2
            .as_array()
            .index_axis(Axis(0), i)
            .into_dimensionality::<Ix2>()
            .unwrap();
        out_data.extend(ai.dot(&bi).iter().copied());
    }
    let mut out_shape = a_batch.to_vec();
    out_shape.push(am);
    out_shape.push(bn);
    Tensor::from_vec(&out_shape, out_data).map_err(LinAlgError::Tensor)
}

/// Matrix multiplication, following NumPy's `matmul` rank rules:
/// - `1D . 1D` is an inner product (returned as a 0-D tensor).
/// - `2D . 1D` / `1D . 2D` are matrix-vector / vector-matrix products.
/// - `2D . 2D` is the ordinary matrix product.
/// - Higher rank is treated as a batch of matrices over the leading axes,
///   which must match exactly between `a` and `b` (no batch-dimension
///   broadcasting).
pub fn matmul<T: LinalgScalar>(a: &Tensor<T>, b: &Tensor<T>) -> LinAlgResult<Tensor<T>> {
    match (a.ndim(), b.ndim()) {
        (1, 1) => {
            let av = to_array1(a, "matmul")?;
            let bv = to_array1(b, "matmul")?;
            if av.len() != bv.len() {
                return Err(LinAlgError::IncompatibleShapes {
                    op: "matmul",
                    lhs: a.shape().to_vec(),
                    rhs: b.shape().to_vec(),
                });
            }
            Tensor::from_vec(&[], vec![av.dot(&bv)]).map_err(LinAlgError::Tensor)
        }
        (2, 1) => {
            let am = to_array2(a, "matmul")?;
            let bv = to_array1(b, "matmul")?;
            if am.ncols() != bv.len() {
                return Err(LinAlgError::IncompatibleShapes {
                    op: "matmul",
                    lhs: a.shape().to_vec(),
                    rhs: b.shape().to_vec(),
                });
            }
            Ok(Tensor::from_owned(am.dot(&bv)))
        }
        (1, 2) => {
            let av = to_array1(a, "matmul")?;
            let bm = to_array2(b, "matmul")?;
            if av.len() != bm.nrows() {
                return Err(LinAlgError::IncompatibleShapes {
                    op: "matmul",
                    lhs: a.shape().to_vec(),
                    rhs: b.shape().to_vec(),
                });
            }
            Ok(Tensor::from_owned(av.dot(&bm)))
        }
        (2, 2) => {
            let am = to_array2(a, "matmul")?;
            let bm = to_array2(b, "matmul")?;
            if am.ncols() != bm.nrows() {
                return Err(LinAlgError::IncompatibleShapes {
                    op: "matmul",
                    lhs: a.shape().to_vec(),
                    rhs: b.shape().to_vec(),
                });
            }
            Ok(Tensor::from_owned(am.dot(&bm)))
        }
        (na, nb) if na >= 2 && nb >= 2 => batched_matmul(a, b),
        (na, nb) => Err(LinAlgError::RankMismatch {
            op: "matmul",
            expected: 2,
            actual: na.max(nb),
        }),
    }
}

/// NumPy-style `tensordot`: contract `a`'s axes in `axes_a` against `b`'s
/// axes in `axes_b` (pairwise, in the given order), summing over them.
/// Implemented by permuting the contracted axes to the boundary,
/// reshaping to 2-D, and delegating to [`matmul`] -- the standard
/// reduction of a general contraction to one matrix product.
pub fn tensordot<T: LinalgScalar>(
    a: &Tensor<T>,
    b: &Tensor<T>,
    axes_a: &[usize],
    axes_b: &[usize],
) -> LinAlgResult<Tensor<T>> {
    if axes_a.len() != axes_b.len() {
        return Err(LinAlgError::InvalidContraction(format!(
            "tensordot: {} contracted axes on the left, {} on the right",
            axes_a.len(),
            axes_b.len()
        )));
    }
    for (&ax_a, &ax_b) in axes_a.iter().zip(axes_b) {
        if ax_a >= a.ndim() || ax_b >= b.ndim() {
            return Err(LinAlgError::InvalidContraction(format!(
                "tensordot: axis {ax_a} or {ax_b} out of bounds"
            )));
        }
        if a.shape()[ax_a] != b.shape()[ax_b] {
            return Err(LinAlgError::IncompatibleShapes {
                op: "tensordot",
                lhs: a.shape().to_vec(),
                rhs: b.shape().to_vec(),
            });
        }
    }

    let a_free: Vec<usize> = (0..a.ndim()).filter(|i| !axes_a.contains(i)).collect();
    let b_free: Vec<usize> = (0..b.ndim()).filter(|i| !axes_b.contains(i)).collect();

    let mut a_perm = a_free.clone();
    a_perm.extend(axes_a.iter().copied());
    let mut b_perm = axes_b.to_vec();
    b_perm.extend(b_free.iter().copied());

    let a_p = a.permute(&a_perm).map_err(LinAlgError::Tensor)?;
    let b_p = b.permute(&b_perm).map_err(LinAlgError::Tensor)?;

    let a_free_size: usize = a_free.iter().map(|&i| a.shape()[i]).product();
    let contract_size: usize = axes_a.iter().map(|&i| a.shape()[i]).product();
    let b_free_size: usize = b_free.iter().map(|&i| b.shape()[i]).product();

    let a2 = a_p
        .reshape(&[a_free_size, contract_size])
        .map_err(LinAlgError::Tensor)?;
    let b2 = b_p
        .reshape(&[contract_size, b_free_size])
        .map_err(LinAlgError::Tensor)?;
    let result2 = matmul(&a2, &b2)?;

    let mut out_shape: Vec<usize> = a_free.iter().map(|&i| a.shape()[i]).collect();
    out_shape.extend(b_free.iter().map(|&i| b.shape()[i]));
    result2.reshape(&out_shape).map_err(LinAlgError::Tensor)
}

/// A minimal `einsum`: `"ij,jk->ik"`-style subscripts over any number of
/// operands, with implicit summation over every label that does not
/// appear in the output. This is a correctness-first, brute-force
/// reference implementation -- it evaluates by iterating the full
/// Cartesian product of every distinct label's size, so it is
/// `O(product of all operand dimensions)` rather than using a
/// contraction-order-optimized path the way a real `einsum` would.
pub fn einsum<T>(pattern: &str, tensors: &[&Tensor<T>]) -> LinAlgResult<Tensor<T>>
where
    T: Copy + Zero + std::ops::Add<Output = T> + std::ops::Mul<Output = T>,
{
    if tensors.is_empty() {
        return Err(LinAlgError::InvalidContraction(
            "einsum needs at least one operand".into(),
        ));
    }
    let (lhs, rhs) = pattern.split_once("->").ok_or_else(|| {
        LinAlgError::InvalidContraction(format!("einsum pattern {pattern:?} must contain '->'"))
    })?;
    let operand_specs: Vec<Vec<char>> = lhs
        .split(',')
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect())
        .collect();
    if operand_specs.len() != tensors.len() {
        return Err(LinAlgError::InvalidContraction(format!(
            "einsum pattern {pattern:?} names {} operands, {} were given",
            operand_specs.len(),
            tensors.len()
        )));
    }
    let out_labels: Vec<char> = rhs.chars().filter(|c| !c.is_whitespace()).collect();

    let mut sizes: HashMap<char, usize> = HashMap::new();
    for (spec, tensor) in operand_specs.iter().zip(tensors) {
        if spec.len() != tensor.ndim() {
            return Err(LinAlgError::InvalidContraction(format!(
                "einsum: operand with {} labels but rank {}",
                spec.len(),
                tensor.ndim()
            )));
        }
        for (&label, &dim) in spec.iter().zip(tensor.shape()) {
            match sizes.get(&label) {
                Some(&sz) if sz != dim => {
                    return Err(LinAlgError::InvalidContraction(format!(
                        "einsum: axis {label:?} has size {sz} in one operand and {dim} in another"
                    )));
                }
                _ => {
                    sizes.insert(label, dim);
                }
            }
        }
    }
    for label in &out_labels {
        if !sizes.contains_key(label) {
            return Err(LinAlgError::InvalidContraction(format!(
                "einsum: output label {label:?} does not appear in any operand"
            )));
        }
    }

    let mut all_labels: Vec<char> = out_labels.clone();
    for spec in &operand_specs {
        for &l in spec {
            if !all_labels.contains(&l) {
                all_labels.push(l);
            }
        }
    }
    let dims: Vec<usize> = all_labels.iter().map(|l| sizes[l]).collect();
    let out_shape: Vec<usize> = out_labels.iter().map(|l| sizes[l]).collect();
    let out_len: usize = out_shape.iter().product::<usize>().max(1);
    let mut out_data = vec![T::zero(); out_len];

    let total: usize = dims.iter().product::<usize>().max(1);
    let mut idx = vec![0usize; all_labels.len()];
    for _ in 0..total {
        let mut term: Option<T> = None;
        for (spec, tensor) in operand_specs.iter().zip(tensors) {
            let elem_idx: Vec<usize> = spec
                .iter()
                .map(|l| idx[all_labels.iter().position(|x| x == l).unwrap()])
                .collect();
            let val = *tensor
                .get(&elem_idx)
                .expect("index within bounds by construction");
            term = Some(match term {
                None => val,
                Some(acc) => acc * val,
            });
        }
        let mut out_flat = 0usize;
        for &l in &out_labels {
            let pos = all_labels.iter().position(|x| *x == l).unwrap();
            out_flat = out_flat * sizes[&l] + idx[pos];
        }
        out_data[out_flat] = out_data[out_flat] + term.expect("at least one operand");

        for k in (0..idx.len()).rev() {
            idx[k] += 1;
            if idx[k] < dims[k] {
                break;
            }
            idx[k] = 0;
        }
    }

    Tensor::from_vec(&out_shape, out_data).map_err(LinAlgError::Tensor)
}
