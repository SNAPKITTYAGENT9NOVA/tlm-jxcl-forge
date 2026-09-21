use tensor_core::{Order, Tensor};

#[test]
fn layout_round_trip_preserves_logical_values() {
    let t = Tensor::from_vec(&[2, 3], (0..6).collect::<Vec<i32>>()).unwrap();
    assert!(t.is_standard_layout());

    let f = t.to_order(Order::Fortran);
    assert!(!f.is_standard_layout());
    assert!(f.is_fortran_layout());
    // Logical element access must be unaffected by the physical layout.
    for i in 0..2 {
        for j in 0..3 {
            assert_eq!(t.get(&[i, j]), f.get(&[i, j]));
        }
    }

    let back = f.to_order(Order::C);
    assert!(back.is_standard_layout());
    assert_eq!(back, t);
}

#[test]
fn reshape_errors_on_element_count_mismatch() {
    let t = Tensor::<f64>::zeros(&[2, 3]);
    assert!(t.reshape(&[3, 3]).is_err());
    assert!(t.reshape(&[6]).is_ok());
    assert!(t.reshape(&[1, 6]).is_ok());
}
