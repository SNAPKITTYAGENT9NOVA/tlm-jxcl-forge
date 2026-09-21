use approx::assert_relative_eq;
use tensor_core::Tensor;

#[test]
fn full_reductions() {
    let t = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    assert_eq!(t.sum(), 21.0);
    assert_eq!(t.mean(), Some(3.5));
    assert_eq!(t.max().unwrap(), 6.0);
    assert_eq!(t.min().unwrap(), 1.0);
    assert_eq!(t.argmax().unwrap(), 5);
    assert_eq!(t.argmin().unwrap(), 0);
}

#[test]
fn axis_reductions() {
    // [[1, 2, 3],
    //  [4, 5, 6]]
    let t = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();

    let sum0 = t.sum_axis(0).unwrap();
    assert_eq!(sum0.shape(), &[3]);
    assert_eq!(sum0.to_vec(), vec![5.0, 7.0, 9.0]);

    let sum1 = t.sum_axis(1).unwrap();
    assert_eq!(sum1.shape(), &[2]);
    assert_eq!(sum1.to_vec(), vec![6.0, 15.0]);

    let max0 = t.max_axis(0).unwrap();
    assert_eq!(max0.to_vec(), vec![4.0, 5.0, 6.0]);

    let argmax1 = t.argmax_axis(1).unwrap();
    assert_eq!(argmax1.to_vec(), vec![2, 2]);
}

#[test]
fn variance_and_std() {
    let t = Tensor::from_vec(&[4], vec![2.0, 4.0, 4.0, 4.0]).unwrap();
    // population variance of [2,4,4,4]: mean=3.5, sq devs = [2.25,0.25,0.25,0.25] -> mean 0.75
    assert_relative_eq!(t.var(0).unwrap(), 0.75, epsilon = 1e-10);
    assert_relative_eq!(t.std(0).unwrap(), 0.75f64.sqrt(), epsilon = 1e-10);

    let t2 = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let var_axis0 = t2.var_axis(0, 0).unwrap();
    // column-wise variance of [[1,2,3],[4,5,6]] with ddof=0: each column has
    // two values differing by 3 -> variance = 2.25 for every column.
    for x in var_axis0.to_vec() {
        assert_relative_eq!(x, 2.25, epsilon = 1e-10);
    }
}

#[test]
fn axis_out_of_bounds_errors() {
    let t = Tensor::<f64>::zeros(&[2, 3]);
    assert!(t.sum_axis(5).is_err());
    assert!(t.mean_axis(5).is_err());
}
