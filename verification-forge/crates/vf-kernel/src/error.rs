//! Everything the kernel's judgments can fail with. Every variant is a
//! reported outcome, never a panic: per this project's hard
//! requirements, the kernel fails closed on anything it cannot
//! positively establish, rather than assuming the best.
use std::fmt;
use vf_core::{ArenaError, Symbol, TermId};
use vf_reducer::ReduceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    Arena(ArenaError),
    Reduce(ReduceError),
    /// A [`vf_core::Term::BoundVar`] escaped every binder that should
    /// have opened it. A term in this state is not well-scoped and is
    /// rejected outright -- it is never a valid axiom/definition/proof
    /// body regardless of what it claims to establish.
    UnexpectedLooseBoundVar,
    UnboundVariable(Symbol),
    UnknownConstant(Symbol),
    NotAFunction {
        head_type: TermId,
    },
    NotASort {
        got: TermId,
    },
    TypeMismatch {
        expected: TermId,
        found: TermId,
    },
}

impl From<ArenaError> for KernelError {
    fn from(e: ArenaError) -> Self {
        KernelError::Arena(e)
    }
}
impl From<ReduceError> for KernelError {
    fn from(e: ReduceError) -> Self {
        KernelError::Reduce(e)
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::Arena(e) => write!(f, "{e}"),
            KernelError::Reduce(e) => write!(f, "{e}"),
            KernelError::UnexpectedLooseBoundVar => {
                write!(
                    f,
                    "encountered a bound variable outside any binder (not well-scoped)"
                )
            }
            KernelError::UnboundVariable(_) => write!(f, "unbound local variable"),
            KernelError::UnknownConstant(_) => write!(f, "reference to an undeclared constant"),
            KernelError::NotAFunction { .. } => {
                write!(f, "applied a term whose type is not a Pi (function) type")
            }
            KernelError::NotASort { .. } => {
                write!(f, "expected a type (a term of sort Prop or Type(n))")
            }
            KernelError::TypeMismatch { .. } => write!(f, "type mismatch"),
        }
    }
}
impl std::error::Error for KernelError {}
