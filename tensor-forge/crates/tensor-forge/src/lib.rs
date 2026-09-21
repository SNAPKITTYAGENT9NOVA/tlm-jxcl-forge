//! `tensor-forge`: the facade over [`tensor_core`] and [`tensor_linalg`]
//! -- depend on this crate alone to get both, plus a combined
//! [`prelude`].
//!
//! ```
//! use tensor_forge::prelude::*;
//!
//! let a = Tensor::from_vec(&[2, 2], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
//! let b = Tensor::<f64>::eye(2);
//! let c = matmul(&a, &b).unwrap();
//! assert_eq!(c, a);
//! ```

pub use tensor_core;
pub use tensor_linalg;

pub use tensor_core::{
    Order, ReduceOp, SliceSpec, Tensor, TensorError, TensorResult, broadcast_shape, einops_reduce,
    ndarray_types, rearrange, repeat,
};
pub use tensor_linalg::{
    DEFAULT_MAX_SWEEPS, DEFAULT_TOL_F64, EigDecomposition, LinAlgError, LinAlgResult,
    LuDecomposition, QrDecomposition, SvdDecomposition, cholesky, det, eig_symmetric, einsum,
    inverse, lu, matmul, qr, solve, svd, tensordot,
};

/// `use tensor_forge::prelude::*;` for everything most callers need from
/// both crates.
pub mod prelude {
    pub use tensor_core::prelude::*;
    pub use tensor_linalg::prelude::*;
}
