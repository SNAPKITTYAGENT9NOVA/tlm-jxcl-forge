use tensor_core::TensorError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum LinAlgError {
    #[error(transparent)]
    Tensor(#[from] TensorError),

    #[error("expected a rank-{expected} tensor for {op}, got rank {actual}")]
    RankMismatch {
        op: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("{op} requires a square matrix, got shape {shape:?}")]
    NotSquare { op: &'static str, shape: Vec<usize> },

    #[error("incompatible shapes for {op}: {lhs:?} and {rhs:?}")]
    IncompatibleShapes {
        op: &'static str,
        lhs: Vec<usize>,
        rhs: Vec<usize>,
    },

    #[error("matrix is singular (or numerically indistinguishable from singular) in {op}")]
    SingularMatrix { op: &'static str },

    #[error("matrix is not symmetric, required by {op}")]
    NotSymmetric { op: &'static str },

    #[error("matrix is not positive-definite, required by {op}")]
    NotPositiveDefinite { op: &'static str },

    #[error("{op} did not converge within {max_iter} iterations")]
    ConvergenceFailure { op: &'static str, max_iter: usize },

    #[error("invalid einsum/tensordot spec: {0}")]
    InvalidContraction(String),
}

pub type LinAlgResult<T> = Result<T, LinAlgError>;
