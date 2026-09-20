/// Convenience prelude for importing commonly-used tensor types and traits.
///
/// # Examples
///
/// ```
/// use jxcl_tensor::prelude::*;
///
/// let t = Tensor::<f32>::zeros(&[2, 3]);
/// ```
pub use crate::tensor::Tensor;
pub use crate::error::{Error, Result};
