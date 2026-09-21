use thiserror::Error;

/// Errors produced by `tensor-core`'s shape-checked operations.
///
/// `tensor-core` never panics on a shape mismatch that originates from
/// caller-supplied data (shapes, indices, axis numbers); those are always
/// reported through this type. Panics are reserved for operator-overload
/// call sites (`+`, `-`, `*`, `/`) where NumPy/ndarray's own convention is
/// to panic on incompatible shapes -- callers who want a `Result` should
/// use the `checked_*` methods instead.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum TensorError {
    #[error("shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
    },

    #[error("cannot broadcast shape {from:?} to {to:?}")]
    BroadcastError { from: Vec<usize>, to: Vec<usize> },

    #[error(
        "cannot reshape tensor of {from_len} elements (shape {from:?}) into shape {to:?} \
         ({to_len} elements)"
    )]
    ReshapeError {
        from: Vec<usize>,
        to: Vec<usize>,
        from_len: usize,
        to_len: usize,
    },

    #[error("axis {axis} out of bounds for tensor of rank {ndim}")]
    AxisOutOfBounds { axis: usize, ndim: usize },

    #[error("index {index} out of bounds for axis {axis} of length {len}")]
    IndexOutOfBounds {
        axis: usize,
        index: isize,
        len: usize,
    },

    #[error("expected a rank-{expected} tensor, got rank {actual}")]
    RankMismatch { expected: usize, actual: usize },

    #[error("operation is undefined on an empty tensor")]
    EmptyTensor,

    #[error("data length {data_len} does not match shape {shape:?} ({shape_len} elements)")]
    DataLengthMismatch {
        shape: Vec<usize>,
        shape_len: usize,
        data_len: usize,
    },

    #[error("invalid slice step: step must be non-zero")]
    InvalidStep,

    #[error("invalid einops pattern: {0}")]
    InvalidPattern(String),
}

pub type TensorResult<T> = Result<T, TensorError>;
