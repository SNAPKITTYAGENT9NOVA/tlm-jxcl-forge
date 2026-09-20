#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub mod error;
pub mod tensor;
pub mod ops;
pub mod linalg;
pub mod reduce;
pub mod prelude;

pub use error::{Error, Result};
pub use tensor::Tensor;
