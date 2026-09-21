use std::collections::HashMap;

use tensor_core::{ReduceOp, Tensor, einops_reduce, rearrange, repeat};

#[test]
fn rearrange_permute() {
    let t = Tensor::from_vec(&[2, 3], (0..6).collect::<Vec<i32>>()).unwrap();
    let out = rearrange(&t, "h w -> w h", &HashMap::new()).unwrap();
    assert_eq!(out.shape(), &[3, 2]);
    assert_eq!(out.to_vec(), vec![0, 3, 1, 4, 2, 5]);
}

#[test]
fn rearrange_merge() {
    // b h w c -> b (h w) c
    let t = Tensor::from_vec(&[1, 2, 3, 1], (0..6).collect::<Vec<i32>>()).unwrap();
    let out = rearrange(&t, "b h w c -> b (h w) c", &HashMap::new()).unwrap();
    assert_eq!(out.shape(), &[1, 6, 1]);
    assert_eq!(out.to_vec(), (0..6).collect::<Vec<i32>>());
}

#[test]
fn rearrange_split_with_known_size() {
    // b (h w) c -> b h w c, given h=2
    let t = Tensor::from_vec(&[1, 6, 1], (0..6).collect::<Vec<i32>>()).unwrap();
    let mut sizes = HashMap::new();
    sizes.insert("h".to_string(), 2usize);
    let out = rearrange(&t, "b (h w) c -> b h w c", &sizes).unwrap();
    assert_eq!(out.shape(), &[1, 2, 3, 1]);
    assert_eq!(out.to_vec(), (0..6).collect::<Vec<i32>>());
}

#[test]
fn rearrange_roundtrip() {
    let t = Tensor::from_vec(&[2, 3, 4], (0..24).collect::<Vec<i32>>()).unwrap();
    let mut sizes = HashMap::new();
    sizes.insert("h".to_string(), 3usize);
    let merged = rearrange(&t, "b h w -> b (h w)", &sizes).unwrap();
    assert_eq!(merged.shape(), &[2, 12]);
    let split_back = rearrange(&merged, "b (h w) -> b h w", &sizes).unwrap();
    assert_eq!(split_back, t);
}

#[test]
fn reduce_sum_and_mean() {
    // b h w c -> b c, summing over h and w
    let t = Tensor::from_vec(&[1, 2, 2, 1], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let out = einops_reduce(&t, "b h w c -> b c", ReduceOp::Sum, &HashMap::new()).unwrap();
    assert_eq!(out.shape(), &[1, 1]);
    assert_eq!(out.to_vec(), vec![10.0]);

    let mean_out = einops_reduce(&t, "b h w c -> b c", ReduceOp::Mean, &HashMap::new()).unwrap();
    assert_eq!(mean_out.to_vec(), vec![2.5]);
}

#[test]
fn repeat_new_axis() {
    let t = Tensor::from_vec(&[2], vec![1.0, 2.0]).unwrap();
    let mut sizes = HashMap::new();
    sizes.insert("c".to_string(), 3usize);
    let out = repeat(&t, "h -> h c", &sizes).unwrap();
    assert_eq!(out.shape(), &[2, 3]);
    assert_eq!(out.to_vec(), vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0]);
}

#[test]
fn repeat_with_literal_size() {
    let t = Tensor::from_vec(&[2], vec![1.0, 2.0]).unwrap();
    let out = repeat(&t, "w -> c w", &HashMap::new());
    // "c" has no known size and no literal -> should error
    assert!(out.is_err());

    let out2 = repeat(&t, "w -> 3 w", &HashMap::new()).unwrap();
    assert_eq!(out2.shape(), &[3, 2]);
    assert_eq!(out2.to_vec(), vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
}

#[test]
fn invalid_patterns_report_errors() {
    let t = Tensor::from_vec(&[2, 3], (0..6).collect::<Vec<i32>>()).unwrap();
    assert!(rearrange(&t, "h w w2 -> h w w2", &HashMap::new()).is_err()); // rank mismatch
    assert!(rearrange(&t, "h w -> h x", &HashMap::new()).is_err()); // unknown axis on rhs
    assert!(rearrange(&t, "h w", &HashMap::new()).is_err()); // no '->'
}
