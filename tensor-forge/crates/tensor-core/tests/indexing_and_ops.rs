use tensor_core::{SliceSpec, Tensor, TensorError};

#[test]
fn indexing_and_get() {
    let t = Tensor::from_vec(&[2, 3], (0..6).collect::<Vec<i32>>()).unwrap();
    assert_eq!(t[&[0, 0][..]], 0);
    assert_eq!(t[&[1, 2][..]], 5);
    assert_eq!(t.get(&[1, 2]), Some(&5));
    assert_eq!(t.get(&[5, 5]), None);
    assert_eq!(t.get(&[0]), None); // wrong rank
}

#[test]
fn slice_dyn_ranges_and_negative_indices() {
    let t = Tensor::from_vec(&[5], (0..5).collect::<Vec<i32>>()).unwrap();

    // arr[1:4]
    let s = t
        .slice_dyn(&[SliceSpec::Range {
            start: Some(1),
            end: Some(4),
            step: 1,
        }])
        .unwrap();
    assert_eq!(s.to_vec(), vec![1, 2, 3]);

    // arr[-2:] -> last two elements
    let s2 = t
        .slice_dyn(&[SliceSpec::Range {
            start: Some(-2),
            end: None,
            step: 1,
        }])
        .unwrap();
    assert_eq!(s2.to_vec(), vec![3, 4]);

    // arr[::-1] -> reversed
    let s3 = t
        .slice_dyn(&[SliceSpec::Range {
            start: None,
            end: None,
            step: -1,
        }])
        .unwrap();
    assert_eq!(s3.to_vec(), vec![4, 3, 2, 1, 0]);

    // arr[-1] -> single index, drops axis (0-D)
    let s4 = t.slice_dyn(&[SliceSpec::Index(-1)]).unwrap();
    assert_eq!(s4.shape(), &[] as &[usize]);
    assert_eq!(s4.to_vec(), vec![4]);
}

#[test]
fn slice_dyn_multi_axis() {
    let t = Tensor::from_vec(&[3, 4], (0..12).collect::<Vec<i32>>()).unwrap();
    // rows 1.., columns ..2
    let s = t
        .slice_dyn(&[
            SliceSpec::Range {
                start: Some(1),
                end: None,
                step: 1,
            },
            SliceSpec::Range {
                start: None,
                end: Some(2),
                step: 1,
            },
        ])
        .unwrap();
    assert_eq!(s.shape(), &[2, 2]);
    assert_eq!(s.to_vec(), vec![4, 5, 8, 9]);
}

#[test]
fn broadcasting_arithmetic() {
    let a = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let b = Tensor::from_vec(&[3], vec![10.0, 20.0, 30.0]).unwrap();
    let c = &a + &b;
    assert_eq!(c.to_vec(), vec![11.0, 22.0, 33.0, 14.0, 25.0, 36.0]);

    let d = a.checked_mul(&b).unwrap();
    assert_eq!(d.to_vec(), vec![10.0, 40.0, 90.0, 40.0, 100.0, 180.0]);

    let bad = Tensor::from_vec(&[4], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    assert!(matches!(
        a.checked_add(&bad),
        Err(TensorError::BroadcastError { .. })
    ));
}

#[test]
fn scalar_ops_and_unary_math() {
    let a = Tensor::from_vec(&[3], vec![1.0, 4.0, 9.0]).unwrap();
    let b = a.clone() * 2.0;
    assert_eq!(b.to_vec(), vec![2.0, 8.0, 18.0]);

    let s = a.sqrt();
    assert_eq!(s.to_vec(), vec![1.0, 2.0, 3.0]);

    let r = Tensor::from_vec(&[3], vec![-1.0, 0.0, 2.0]).unwrap().relu();
    assert_eq!(r.to_vec(), vec![0.0, 0.0, 2.0]);
}

#[test]
fn mul_add_fused() {
    let a = Tensor::from_vec(&[2], vec![2.0, 3.0]).unwrap();
    let b = Tensor::from_vec(&[2], vec![4.0, 5.0]).unwrap();
    let c = Tensor::from_vec(&[2], vec![1.0, 1.0]).unwrap();
    let out = a.mul_add(&b, &c).unwrap();
    assert_eq!(out.to_vec(), vec![9.0, 16.0]);
}

#[test]
fn transpose_permute_reshape() {
    let t = Tensor::from_vec(&[2, 3], (0..6).collect::<Vec<i32>>()).unwrap();
    let tt = t.transpose();
    assert_eq!(tt.shape(), &[3, 2]);
    assert_eq!(tt.to_vec(), vec![0, 3, 1, 4, 2, 5]);

    let r = t.reshape(&[3, 2]).unwrap();
    assert_eq!(r.to_vec(), vec![0, 1, 2, 3, 4, 5]);
    assert!(t.reshape(&[4, 4]).is_err());

    let t3 = Tensor::from_vec(&[2, 3, 4], (0..24).collect::<Vec<i32>>()).unwrap();
    let p = t3.permute(&[2, 0, 1]).unwrap();
    assert_eq!(p.shape(), &[4, 2, 3]);
    assert!(t3.permute(&[0, 0, 1]).is_err());
}

#[test]
fn squeeze_and_insert_axis() {
    let t = Tensor::from_vec(&[1, 3, 1], vec![1, 2, 3]).unwrap();
    assert_eq!(t.squeeze().shape(), &[3]);

    let u = Tensor::from_vec(&[3], vec![1, 2, 3]).unwrap();
    let w = u.insert_axis(0).unwrap();
    assert_eq!(w.shape(), &[1, 3]);
}
