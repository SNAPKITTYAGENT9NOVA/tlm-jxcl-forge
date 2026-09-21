//! `tensor-linalg`: linear algebra on top of `tensor-core`'s [`Tensor`]
//! type -- `matmul`/`tensordot`/a reference `einsum`, and pure-Rust
//! LU/QR/Cholesky/Jacobi-eigenvalue/SVD decompositions for `f32`/`f64`.
//!
//! Every decomposition here is a textbook, correctness-first pure-Rust
//! implementation (partial-pivoted Gaussian elimination, Householder
//! reflections, cyclic/one-sided Jacobi rotations) rather than a binding
//! to LAPACK -- see the crate README for when you'd want to reach for
//! `ndarray-linalg`'s BLAS/LAPACK backends instead.

mod cholesky;
mod eig;
mod error;
mod lu;
mod matmul;
mod qr;
mod svd;
mod util;

pub use cholesky::cholesky;
pub use eig::{EigDecomposition, eig_symmetric};
pub use error::{LinAlgError, LinAlgResult};
pub use lu::{LuDecomposition, det, inverse, lu, solve};
pub use matmul::{einsum, matmul, tensordot};
pub use qr::{QrDecomposition, qr};
pub use svd::{SvdDecomposition, svd};

/// Default convergence tolerance and sweep cap for the iterative
/// (Jacobi) decompositions, tuned for `f64`. Pass your own for `f32` or
/// for ill-conditioned inputs.
pub const DEFAULT_TOL_F64: f64 = 1e-12;
pub const DEFAULT_MAX_SWEEPS: usize = 100;

pub mod prelude {
    pub use crate::{
        DEFAULT_MAX_SWEEPS, DEFAULT_TOL_F64, EigDecomposition, LinAlgError, LinAlgResult,
        LuDecomposition, QrDecomposition, SvdDecomposition, cholesky, det, eig_symmetric, einsum,
        inverse, lu, matmul, qr, solve, svd, tensordot,
    };
}
