use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch { expected: Vec<usize>, actual: Vec<usize> },

    #[error("rank mismatch: expected rank {expected}, got rank {actual}")]
    RankMismatch { expected: usize, actual: usize },

    #[error("broadcast shapes {lhs:?} and {rhs:?} are incompatible")]
    BroadcastIncompatible { lhs: Vec<usize>, rhs: Vec<usize> },

    #[error("axis {axis} out of bounds for rank {rank}")]
    AxisOutOfBounds { axis: usize, rank: usize },

    #[error("dimension mismatch in matmul: cannot multiply ({lhs_m}, {lhs_n}) by ({rhs_m}, {rhs_n})")]
    MatmulDimMismatch {
        lhs_m: usize,
        lhs_n: usize,
        rhs_m: usize,
        rhs_n: usize,
    },

    #[error("empty tensor: operation requires non-empty input")]
    EmptyTensor,

    #[error("non-square matrix: expected shape ({n}, {n}), got ({m}, {n})")]
    NonSquareMatrix { m: usize, n: usize },

    #[error("singular matrix: determinant is zero or near-zero")]
    SingularMatrix,

    #[error("index {index} out of bounds for dimension {dim}")]
    IndexOutOfBounds { index: isize, dim: usize },

    #[error("zero division in normalization")]
    ZeroDivision,

    #[error("invalid stride configuration")]
    InvalidStride,

    #[error("operation not implemented: {0}")]
    NotImplemented(String),
}
