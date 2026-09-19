use std::fmt;
use vf_core::Symbol;
use vf_kernel::KernelError;

/// Everything that can make a declaration attempt fail. Never a panic:
/// a policy violation or a kernel rejection is reported, not
/// `unwrap()`-ed past -- registering a bad declaration simply doesn't
/// happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// `name` is already declared. Redeclaration under a new meaning
    /// is never allowed: it would let a later declaration silently
    /// change what an earlier proof's dependencies actually mean.
    DuplicateName(Symbol),
    /// [`crate::AxiomPolicy::NoAxioms`] is in force; `declare_axiom`
    /// was called anyway.
    AxiomsForbidden(Symbol),
    /// [`crate::AxiomPolicy::ExplicitAxiomsOnly`] is in force and no
    /// (non-empty) justification was given for `name`.
    MissingJustification(Symbol),
    /// The kernel independently rejected this declaration -- a
    /// definition/theorem's value doesn't check against its stated
    /// type, or the type itself isn't well-formed.
    Kernel(KernelError),
}

impl From<KernelError> for RegistryError {
    fn from(e: KernelError) -> Self {
        RegistryError::Kernel(e)
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::DuplicateName(_) => {
                write!(f, "a declaration with this name already exists")
            }
            RegistryError::AxiomsForbidden(_) => {
                write!(
                    f,
                    "axioms are forbidden under the current AxiomPolicy::NoAxioms policy"
                )
            }
            RegistryError::MissingJustification(_) => {
                write!(f, "this axiom policy requires a non-empty justification")
            }
            RegistryError::Kernel(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for RegistryError {}
