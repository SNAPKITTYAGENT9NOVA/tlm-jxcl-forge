//! `tensor-core`: an arbitrary-rank [`Tensor`] type built directly on
//! [`ndarray`], covering creation, indexing/slicing (with negative
//! indices and steps), broadcasting, elementwise arithmetic, reductions,
//! memory layout control, and `einops`-style `rearrange`/`reduce`/
//! `repeat`.
//!
//! This crate deliberately does not include linear algebra (matmul,
//! decompositions, `einsum`) -- see the sibling `tensor-linalg` crate for
//! that, or depend on the `tensor-forge` facade crate for both plus a
//! shared prelude.

mod axes;
mod creation;
mod einops;
mod error;
mod indexing;
mod layout;
mod ops;
mod reduce;
mod tensor;

#[cfg(feature = "parallel")]
mod parallel;

#[cfg(feature = "serde")]
mod serde_support;

pub use einops::{ReduceOp, ReducibleAxis, rearrange, reduce as einops_reduce, repeat};
pub use error::{TensorError, TensorResult};
pub use indexing::SliceSpec;
pub use layout::Order;
pub use ops::broadcast_shape;
pub use tensor::Tensor;

/// Re-exports of the underlying `ndarray` types, for interop with code
/// that wants to drop down to them directly.
pub mod ndarray_types {
    pub use ndarray::{
        Array, Array1, Array2, ArrayD, ArrayView, ArrayViewD, ArrayViewMut, ArrayViewMutD, Axis,
        Dimension, IxDyn,
    };
}

/// `use tensor_core::prelude::*;` for the common types and functions.
pub mod prelude {
    pub use crate::{
        Order, ReduceOp, SliceSpec, Tensor, TensorError, TensorResult, broadcast_shape,
        einops_reduce, rearrange, repeat,
    };
}
