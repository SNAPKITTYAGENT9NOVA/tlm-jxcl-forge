use tensor_core::Tensor;
use tensor_linalg::{einsum, matmul, tensordot};

#[test]
fn matmul_2d_2d() {
    let a = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let b = Tensor::from_vec(&[3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
    let c = matmul(&a, &b).unwrap();
    assert_eq!(c.shape(), &[2, 2]);
    // [[1,2,3],[4,5,6]] @ [[7,8],[9,10],[11,12]]
    assert_eq!(c.to_vec(), vec![58.0, 64.0, 139.0, 154.0]);
}

#[test]
fn matmul_vector_forms() {
    let a = Tensor::from_vec(&[3], vec![1.0, 2.0, 3.0]).unwrap();
    let b = Tensor::from_vec(&[3], vec![4.0, 5.0, 6.0]).unwrap();
    let dot = matmul(&a, &b).unwrap();
    assert_eq!(dot.shape(), &[] as &[usize]);
    assert_eq!(dot.to_vec(), vec![32.0]);

    let m = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let mv = matmul(&m, &b).unwrap();
    assert_eq!(mv.to_vec(), vec![32.0, 77.0]);
}

#[test]
fn matmul_batched() {
    let a = Tensor::from_vec(&[2, 2, 2], (1..=8).map(|x| x as f64).collect()).unwrap();
    let b = Tensor::from_vec(&[2, 2, 2], vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0]).unwrap();
    // batched matmul against identity should be a no-op
    let c = matmul(&a, &b).unwrap();
    assert_eq!(c, a);
}

#[test]
fn matmul_shape_errors() {
    let a = Tensor::<f64>::zeros(&[2, 3]);
    let b = Tensor::<f64>::zeros(&[4, 5]);
    assert!(matmul(&a, &b).is_err());
}

#[test]
fn tensordot_matches_matmul() {
    let a = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let b = Tensor::from_vec(&[3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
    let via_matmul = matmul(&a, &b).unwrap();
    let via_tensordot = tensordot(&a, &b, &[1], &[0]).unwrap();
    assert_eq!(via_matmul, via_tensordot);
}

#[test]
fn einsum_matmul() {
    let a = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let b = Tensor::from_vec(&[3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
    let out = einsum("ij,jk->ik", &[&a, &b]).unwrap();
    assert_eq!(out.to_vec(), vec![58.0, 64.0, 139.0, 154.0]);
}

#[test]
fn einsum_transpose_and_trace() {
    let a = Tensor::from_vec(&[2, 3], (0..6).map(|x| x as f64).collect()).unwrap();
    let t = einsum("ij->ji", &[&a]).unwrap();
    assert_eq!(t.shape(), &[3, 2]);
    assert_eq!(t.to_vec(), a.transpose().to_vec());

    let sq = Tensor::from_vec(&[3, 3], (0..9).map(|x| x as f64).collect()).unwrap();
    let trace = einsum("ii->", &[&sq]).unwrap();
    assert_eq!(trace.to_vec(), vec![0.0 + 4.0 + 8.0]);
}

#[test]
fn einsum_diagonal_extraction() {
    let sq = Tensor::from_vec(&[3, 3], (0..9).map(|x| x as f64).collect()).unwrap();
    let diag = einsum("ii->i", &[&sq]).unwrap();
    assert_eq!(diag.to_vec(), vec![0.0, 4.0, 8.0]);
}
