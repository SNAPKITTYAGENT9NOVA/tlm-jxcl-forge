use tensor_core::Tensor;

#[test]
fn zeros_ones_full() {
    let z = Tensor::<f64>::zeros(&[2, 3]);
    assert_eq!(z.shape(), &[2, 3]);
    assert_eq!(z.sum(), 0.0);

    let o = Tensor::<f64>::ones(&[2, 3]);
    assert_eq!(o.sum(), 6.0);

    let f = Tensor::<i32>::full(&[3], 7);
    assert_eq!(f.to_vec(), vec![7, 7, 7]);
}

#[test]
fn eye() {
    let e = Tensor::<f64>::eye(3);
    assert_eq!(e.shape(), &[3, 3]);
    assert_eq!(
        e.to_vec(),
        vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn arange_and_linspace() {
    let a = Tensor::<f64>::arange(0.0, 5.0, 1.0).unwrap();
    assert_eq!(a.to_vec(), vec![0.0, 1.0, 2.0, 3.0, 4.0]);

    let a2 = Tensor::<f64>::arange(10.0, 0.0, -2.0).unwrap();
    assert_eq!(a2.to_vec(), vec![10.0, 8.0, 6.0, 4.0, 2.0]);

    let ls = Tensor::<f64>::linspace(0.0, 1.0, 5);
    assert_eq!(ls.to_vec(), vec![0.0, 0.25, 0.5, 0.75, 1.0]);

    assert!(Tensor::<f64>::arange(0.0, 5.0, 0.0).is_err());
}

#[test]
fn from_shape_fn_and_from_vec() {
    let t = Tensor::from_shape_fn(&[2, 2], |idx| (idx[0] * 10 + idx[1]) as i64);
    assert_eq!(t.to_vec(), vec![0, 1, 10, 11]);

    let v = Tensor::from_vec(&[2, 3], (0..6).collect()).unwrap();
    assert_eq!(v.shape(), &[2, 3]);
    assert_eq!(v.to_vec(), (0..6).collect::<Vec<i32>>());

    assert!(Tensor::from_vec(&[2, 3], vec![0; 5]).is_err());
}

#[cfg(feature = "rand")]
#[test]
fn random_respects_shape_and_bounds() {
    let t = Tensor::<f64>::random_uniform(&[4, 5], -1.0, 1.0);
    assert_eq!(t.shape(), &[4, 5]);
    assert!(t.to_vec().iter().all(|&x| (-1.0..1.0).contains(&x)));

    let n = Tensor::<f64>::random_normal(&[100], 0.0, 1.0).unwrap();
    assert_eq!(n.len(), 100);
}
