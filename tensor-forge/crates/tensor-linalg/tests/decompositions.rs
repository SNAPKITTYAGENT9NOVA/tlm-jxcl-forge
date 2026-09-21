use approx::assert_relative_eq;
use tensor_core::Tensor;
use tensor_linalg::{cholesky, det, eig_symmetric, inverse, lu, matmul, qr, solve, svd};

fn mat(shape: &[usize], data: Vec<f64>) -> Tensor<f64> {
    Tensor::from_vec(shape, data).unwrap()
}

fn assert_tensors_close(a: &Tensor<f64>, b: &Tensor<f64>, eps: f64) {
    assert_eq!(a.shape(), b.shape());
    for (x, y) in a.to_vec().into_iter().zip(b.to_vec()) {
        assert_relative_eq!(x, y, epsilon = eps);
    }
}

#[test]
fn lu_reconstructs_a() {
    let a = mat(&[3, 3], vec![2.0, 1.0, 1.0, 4.0, 3.0, 3.0, 8.0, 7.0, 9.0]);
    let decomp = lu(&a).unwrap();
    let pa = matmul(&decomp.p, &a).unwrap();
    let lu_product = matmul(&decomp.l, &decomp.u).unwrap();
    assert_tensors_close(&pa, &lu_product, 1e-9);
}

#[test]
fn solve_and_det_and_inverse() {
    let a = mat(&[3, 3], vec![2.0, 1.0, 1.0, 4.0, 3.0, 3.0, 8.0, 7.0, 9.0]);
    let b = mat(&[3], vec![4.0, 10.0, 24.0]);
    let x = solve(&a, &b).unwrap();
    let check = matmul(&a, &x).unwrap();
    assert_tensors_close(&check, &b, 1e-9);

    let d = det(&a).unwrap();
    assert_relative_eq!(d, 4.0, epsilon = 1e-9);

    let inv = inverse(&a).unwrap();
    let identity = matmul(&a, &inv).unwrap();
    let eye = Tensor::<f64>::eye(3);
    assert_tensors_close(&identity, &eye, 1e-9);
}

#[test]
fn singular_matrix_is_reported() {
    let a = mat(&[2, 2], vec![1.0, 2.0, 2.0, 4.0]);
    assert!(lu(&a).is_err());
    assert_eq!(det(&a).unwrap(), 0.0);
    assert!(inverse(&a).is_err());
}

#[test]
fn qr_reconstructs_a_and_is_orthogonal() {
    let a = mat(
        &[3, 3],
        vec![12.0, -51.0, 4.0, 6.0, 167.0, -68.0, -4.0, 24.0, -41.0],
    );
    let decomp = qr(&a).unwrap();
    let qr_product = matmul(&decomp.q, &decomp.r).unwrap();
    assert_tensors_close(&qr_product, &a, 1e-8);

    let qtq = matmul(&decomp.q.transpose(), &decomp.q).unwrap();
    let eye = Tensor::<f64>::eye(3);
    assert_tensors_close(&qtq, &eye, 1e-8);
}

#[test]
fn qr_on_rectangular_matrix() {
    let a = mat(&[4, 2], vec![1.0, 1.0, 1.0, 2.0, 1.0, 3.0, 1.0, 4.0]);
    let decomp = qr(&a).unwrap();
    assert_eq!(decomp.q.shape(), &[4, 4]);
    assert_eq!(decomp.r.shape(), &[4, 2]);
    let qr_product = matmul(&decomp.q, &decomp.r).unwrap();
    assert_tensors_close(&qr_product, &a, 1e-8);
}

#[test]
fn cholesky_reconstructs_spd_matrix() {
    // A = [[4,12,-16],[12,37,-43],[-16,-43,98]], known SPD with L given by
    // the classic textbook example.
    let a = mat(
        &[3, 3],
        vec![4.0, 12.0, -16.0, 12.0, 37.0, -43.0, -16.0, -43.0, 98.0],
    );
    let l = cholesky(&a).unwrap();
    let llt = matmul(&l, &l.transpose()).unwrap();
    assert_tensors_close(&llt, &a, 1e-8);
}

#[test]
fn cholesky_rejects_non_spd() {
    let not_symmetric = mat(&[2, 2], vec![1.0, 2.0, 3.0, 4.0]);
    assert!(cholesky(&not_symmetric).is_err());

    let not_pd = mat(&[2, 2], vec![1.0, 2.0, 2.0, 1.0]);
    assert!(cholesky(&not_pd).is_err());
}

#[test]
fn eig_symmetric_matches_known_eigenvalues() {
    // A 2x2 symmetric matrix [[2,1],[1,2]] has eigenvalues 1 and 3.
    let a = mat(&[2, 2], vec![2.0, 1.0, 1.0, 2.0]);
    let decomp = eig_symmetric(&a, 100, 1e-12).unwrap();
    let values = decomp.values.to_vec();
    assert_relative_eq!(values[0], 3.0, epsilon = 1e-9);
    assert_relative_eq!(values[1], 1.0, epsilon = 1e-9);

    // A v = lambda v for each eigenpair.
    for i in 0..2 {
        let v = decomp
            .vectors
            .slice_dyn(&[
                tensor_core::SliceSpec::full(),
                tensor_core::SliceSpec::Index(i as isize),
            ])
            .unwrap();
        let av = matmul(&a, &v).unwrap();
        let lambda_v = v.clone() * values[i as usize];
        assert_tensors_close(&av, &lambda_v, 1e-8);
    }
}

#[test]
fn eig_symmetric_rejects_asymmetric_input() {
    let a = mat(&[2, 2], vec![1.0, 2.0, 3.0, 4.0]);
    assert!(eig_symmetric(&a, 100, 1e-12).is_err());
}

#[test]
fn svd_reconstructs_a() {
    let a = mat(&[4, 2], vec![2.0, 4.0, 1.0, 3.0, 0.0, 0.0, 0.0, 0.0]);
    let decomp = svd(&a, 200, 1e-13).unwrap();

    // Singular values sorted descending and non-negative.
    let s = decomp.s.to_vec();
    assert!(s.windows(2).all(|w| w[0] >= w[1]));
    assert!(s.iter().all(|&x| x >= 0.0));

    // U diag(S) V^T == A.
    let sdiag = Tensor::from_shape_fn(
        &[2, 2],
        |idx| if idx[0] == idx[1] { s[idx[0]] } else { 0.0 },
    );
    let us = matmul(&decomp.u, &sdiag).unwrap();
    let reconstructed = matmul(&us, &decomp.v.transpose()).unwrap();
    assert_tensors_close(&reconstructed, &a, 1e-8);
}

#[test]
fn svd_handles_wide_matrices_via_transpose() {
    let a = mat(&[2, 4], vec![2.0, 4.0, 1.0, 3.0, 0.0, 0.0, 0.0, 0.0]);
    let decomp = svd(&a, 200, 1e-13).unwrap();
    assert_eq!(decomp.u.shape(), &[2, 2]);
    assert_eq!(decomp.v.shape(), &[4, 2]);
}
