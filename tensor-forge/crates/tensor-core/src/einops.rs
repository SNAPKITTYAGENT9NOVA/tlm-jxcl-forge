//! `einops`-style `rearrange`/`reduce`/`repeat`, built on top of
//! [`Tensor::reshape`] and [`Tensor::permute`].
//!
//! Patterns look like `"b h w c -> b c h w"`: space-separated axis names
//! on each side of `->`, with `(a b)` grouping several axis names into one
//! physical axis (a merge on the output side, a split on the input side).
//! This is a deliberately smaller grammar than the Python `einops`
//! package's (no ellipsis `...`, no anonymous axes besides bare integer
//! literals on the output side) -- it covers permute, split, merge,
//! axis-dropping reduction, and axis-adding repeat, which is the bulk of
//! what `einops` is reached for in practice.

use std::collections::HashMap;

use crate::error::{TensorError, TensorResult};
use crate::tensor::Tensor;

/// The reduction applied to axes present on the left of a [`reduce`]
/// pattern but not on the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceOp {
    Sum,
    Mean,
    Max,
    Min,
}

type Groups = Vec<Vec<String>>;

fn parse_side(pattern: &str, side: &str) -> TensorResult<Groups> {
    let mut groups = Groups::new();
    let mut rest = side;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some(after_paren) = rest.strip_prefix('(') {
            let end = after_paren.find(')').ok_or_else(|| {
                TensorError::InvalidPattern(format!("unclosed '(' in pattern {pattern:?}"))
            })?;
            let inner = &after_paren[..end];
            let names: Vec<String> = inner.split_whitespace().map(str::to_string).collect();
            if names.is_empty() {
                return Err(TensorError::InvalidPattern(format!(
                    "empty group '()' in pattern {pattern:?}"
                )));
            }
            groups.push(names);
            rest = &after_paren[end + 1..];
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(rest.len());
            groups.push(vec![rest[..end].to_string()]);
            rest = &rest[end..];
        }
    }
    Ok(groups)
}

fn split_pattern(pattern: &str) -> TensorResult<(Groups, Groups)> {
    let mut sides = pattern.split("->");
    let (Some(lhs), Some(rhs), None) = (sides.next(), sides.next(), sides.next()) else {
        return Err(TensorError::InvalidPattern(format!(
            "pattern {pattern:?} must contain exactly one '->'"
        )));
    };
    Ok((parse_side(pattern, lhs)?, parse_side(pattern, rhs)?))
}

/// Split the tensor so every input group becomes one axis per name in that
/// group, returning the split tensor and the flat, in-order list of
/// elementary axis names (with their resolved sizes recorded into
/// `known_sizes`).
fn split_input<T: Clone>(
    tensor: &Tensor<T>,
    lhs: &Groups,
    known_sizes: &mut HashMap<String, usize>,
) -> TensorResult<(Tensor<T>, Vec<String>)> {
    if lhs.len() != tensor.ndim() {
        return Err(TensorError::RankMismatch {
            expected: lhs.len(),
            actual: tensor.ndim(),
        });
    }
    let mut flat_names = Vec::new();
    let mut split_shape = Vec::new();
    for (axis, group) in lhs.iter().enumerate() {
        let axis_len = tensor.shape()[axis];
        if group.len() == 1 {
            known_sizes.insert(group[0].clone(), axis_len);
            flat_names.push(group[0].clone());
            split_shape.push(axis_len);
            continue;
        }
        let mut unknown: Option<usize> = None;
        let mut known_product = 1usize;
        for (i, name) in group.iter().enumerate() {
            match known_sizes.get(name) {
                Some(&sz) => known_product *= sz,
                None if unknown.is_none() => unknown = Some(i),
                None => {
                    return Err(TensorError::InvalidPattern(format!(
                        "group {group:?} has more than one axis of unknown size; pass its size in `sizes`"
                    )));
                }
            }
        }
        if let Some(i) = unknown {
            if known_product == 0 || !axis_len.is_multiple_of(known_product) {
                return Err(TensorError::InvalidPattern(format!(
                    "cannot split axis of length {axis_len} into group {group:?} with known sizes multiplying to {known_product}"
                )));
            }
            known_sizes.insert(group[i].clone(), axis_len / known_product);
        } else if known_product != axis_len {
            return Err(TensorError::InvalidPattern(format!(
                "group {group:?} sizes multiply to {known_product}, but the axis has length {axis_len}"
            )));
        }
        for name in group {
            let sz = known_sizes[name];
            flat_names.push(name.clone());
            split_shape.push(sz);
        }
    }
    Ok((tensor.reshape(&split_shape)?, flat_names))
}

fn permutation_for(
    flat_names: &[String],
    target: &[String],
    pattern: &str,
) -> TensorResult<Vec<usize>> {
    target
        .iter()
        .map(|name| {
            flat_names.iter().position(|n| n == name).ok_or_else(|| {
                TensorError::InvalidPattern(format!(
                    "axis {name:?} on the right of {pattern:?} does not appear on the left"
                ))
            })
        })
        .collect()
}

/// Reshape a tensor whose axes are exactly `order` (one axis per name, in
/// that order) into `rhs` groups, merging each group's axes by their
/// known sizes.
fn merge_output<T: Clone>(
    tensor: &Tensor<T>,
    order: &[String],
    rhs: &Groups,
    known_sizes: &HashMap<String, usize>,
) -> TensorResult<Tensor<T>> {
    debug_assert_eq!(tensor.ndim(), order.len());
    let mut out_shape = Vec::with_capacity(rhs.len());
    for group in rhs {
        let mut product = 1usize;
        for name in group {
            product *= known_sizes
                .get(name)
                .copied()
                .or_else(|| name.parse().ok())
                .ok_or_else(|| {
                    TensorError::InvalidPattern(format!("unknown size for axis {name:?}"))
                })?;
        }
        out_shape.push(product);
    }
    tensor.reshape(&out_shape)
}

/// `rearrange`: permute, split, and/or merge axes with no change in the
/// set of elements -- every name on the left must appear exactly once on
/// the right, and vice versa.
///
/// `sizes` gives the size of any axis introduced by a split group (e.g.
/// `"b (h w) c -> b h w c"` needs at least one of `h`/`w`'s size; the
/// other is inferred).
pub fn rearrange<T: Clone>(
    tensor: &Tensor<T>,
    pattern: &str,
    sizes: &HashMap<String, usize>,
) -> TensorResult<Tensor<T>> {
    let (lhs, rhs) = split_pattern(pattern)?;
    let mut known = sizes.clone();
    let (split, flat_names) = split_input(tensor, &lhs, &mut known)?;

    let rhs_flat: Vec<String> = rhs.iter().flatten().cloned().collect();
    if rhs_flat.len() != flat_names.len() {
        return Err(TensorError::InvalidPattern(format!(
            "pattern {pattern:?}: {} axes on the left, {} on the right",
            flat_names.len(),
            rhs_flat.len()
        )));
    }
    let perm = permutation_for(&flat_names, &rhs_flat, pattern)?;
    let permuted = split.permute(&perm)?;
    merge_output(&permuted, &rhs_flat, &rhs, &known)
}

/// `reduce`: like [`rearrange`], but axis names that appear on the left
/// and not on the right are collapsed with `op` instead of being an error.
pub fn reduce<T>(
    tensor: &Tensor<T>,
    pattern: &str,
    op: ReduceOp,
    sizes: &HashMap<String, usize>,
) -> TensorResult<Tensor<T>>
where
    T: Clone,
    Tensor<T>: ReducibleAxis<T>,
{
    let (lhs, rhs) = split_pattern(pattern)?;
    let mut known = sizes.clone();
    let (split, flat_names) = split_input(tensor, &lhs, &mut known)?;

    let rhs_flat: Vec<String> = rhs.iter().flatten().cloned().collect();
    for name in &rhs_flat {
        if !flat_names.contains(name) {
            return Err(TensorError::InvalidPattern(format!(
                "axis {name:?} on the right of {pattern:?} does not appear on the left"
            )));
        }
    }

    let mut reduced = split;
    let mut kept_names = Vec::new();
    for (axis, name) in flat_names.iter().enumerate().rev() {
        if !rhs_flat.contains(name) {
            reduced = reduced.reduce_axis(axis, op)?;
        } else {
            kept_names.push(name.clone());
        }
    }
    kept_names.reverse();

    let perm = permutation_for(&kept_names, &rhs_flat, pattern)?;
    let permuted = reduced.permute(&perm)?;
    merge_output(&permuted, &rhs_flat, &rhs, &known)
}

/// `repeat`: like [`rearrange`], but axis names that appear on the right
/// and not on the left are inserted as new, broadcast axes instead of
/// being an error. New axes need a size, given either in `sizes` or as a
/// bare integer literal in the pattern (e.g. `"h w -> h w 3"`).
pub fn repeat<T: Clone>(
    tensor: &Tensor<T>,
    pattern: &str,
    sizes: &HashMap<String, usize>,
) -> TensorResult<Tensor<T>> {
    let (lhs, rhs) = split_pattern(pattern)?;
    let mut known = sizes.clone();
    let (split, flat_names) = split_input(tensor, &lhs, &mut known)?;

    let rhs_flat: Vec<String> = rhs.iter().flatten().cloned().collect();
    let existing: Vec<String> = rhs_flat
        .iter()
        .filter(|n| flat_names.contains(n))
        .cloned()
        .collect();
    if existing.len() != flat_names.len() {
        return Err(TensorError::InvalidPattern(format!(
            "pattern {pattern:?}: every axis on the left of `repeat` must appear on the right (use `reduce` to drop axes)"
        )));
    }

    let perm = permutation_for(&flat_names, &existing, pattern)?;
    let mut current = split.permute(&perm)?;

    // Insert a length-1 axis for every new name, left to right, then
    // broadcast the whole thing to the target shape in one shot.
    let mut shape_names = existing;
    let mut target_shape = current.shape().to_vec();
    for (pos, name) in rhs_flat.iter().enumerate() {
        if !shape_names.contains(name) {
            let size = known.get(name).copied().or_else(|| name.parse().ok()).ok_or_else(|| {
                TensorError::InvalidPattern(format!(
                    "axis {name:?} is new on the right of {pattern:?}; give its size via `sizes` or as a literal"
                ))
            })?;
            current = current.insert_axis(pos)?;
            shape_names.insert(pos, name.clone());
            target_shape.insert(pos, size);
            known.insert(name.clone(), size);
        }
    }
    let broadcasted = current.broadcast_to(&target_shape)?;
    merge_output(&broadcasted, &rhs_flat, &rhs, &known)
}

/// Internal: lets [`reduce`] collapse one axis with whichever `ReduceOp`
/// it was asked for, generically over the element type's actual
/// arithmetic/ordering bounds.
pub trait ReducibleAxis<T> {
    fn reduce_axis(&self, axis: usize, op: ReduceOp) -> TensorResult<Tensor<T>>;
}

impl<T> ReducibleAxis<T> for Tensor<T>
where
    T: num_traits::Float + num_traits::FromPrimitive,
{
    fn reduce_axis(&self, axis: usize, op: ReduceOp) -> TensorResult<Tensor<T>> {
        match op {
            ReduceOp::Sum => self.sum_axis(axis),
            ReduceOp::Mean => self.mean_axis(axis),
            ReduceOp::Max => self.max_axis(axis),
            ReduceOp::Min => self.min_axis(axis),
        }
    }
}
