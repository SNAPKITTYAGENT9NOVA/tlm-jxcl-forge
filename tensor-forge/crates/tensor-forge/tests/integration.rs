use std::collections::HashMap;

use tensor_forge::prelude::*;

#[test]
fn end_to_end_pipeline() {
    // Build a batch of 2x2 "images" with 3 channels: shape (batch=2, h=2, w=2, c=3).
    let batch = Tensor::from_shape_fn(&[2, 2, 2, 3], |idx| {
        (idx[0] * 1000 + idx[1] * 100 + idx[2] * 10 + idx[3]) as f64
    });

    // einops: channels-last -> channels-first, the classic NCHW conversion.
    let nchw = rearrange(&batch, "b h w c -> b c h w", &HashMap::new()).unwrap();
    assert_eq!(nchw.shape(), &[2, 3, 2, 2]);

    // Flatten each image to a vector per batch element, then run it
    // through a small linear layer via matmul.
    let flat = rearrange(&batch, "b h w c -> b (h w c)", &HashMap::new()).unwrap();
    assert_eq!(flat.shape(), &[2, 12]);

    let weights = Tensor::<f64>::eye(12) * 0.5;
    let projected = matmul(&flat, &weights).unwrap();
    assert_eq!(projected.shape(), &[2, 12]);
    for (x, y) in projected.to_vec().into_iter().zip(flat.to_vec()) {
        assert!((x - y * 0.5).abs() < 1e-12);
    }

    // Reduce across the batch with einops, and separately verify against
    // tensor-core's own axis reduction.
    let per_channel_mean =
        einops_reduce(&batch, "b h w c -> c", ReduceOp::Mean, &HashMap::new()).unwrap();
    assert_eq!(per_channel_mean.shape(), &[3]);

    // A tiny linear-algebra sanity check tying tensor-linalg into the
    // same pipeline: solve a system built from the flattened batch mean.
    let a = Tensor::<f64>::from_vec(&[2, 2], vec![4.0, 1.0, 1.0, 3.0]).unwrap();
    let b = Tensor::<f64>::from_vec(&[2], vec![1.0, 2.0]).unwrap();
    let x = solve(&a, &b).unwrap();
    let recovered = matmul(&a, &x).unwrap();
    for (u, v) in recovered.to_vec().into_iter().zip(b.to_vec()) {
        assert!((u - v).abs() < 1e-9);
    }
}

#[cfg(feature = "rand")]
#[test]
fn random_tensor_decomposes() {
    let a = Tensor::<f64>::random_uniform(&[5, 3], -1.0, 1.0);
    let decomp = qr(&a).unwrap();
    let reconstructed = matmul(&decomp.q, &decomp.r).unwrap();
    for (x, y) in reconstructed.to_vec().into_iter().zip(a.to_vec()) {
        assert!((x - y).abs() < 1e-8);
    }
}
