//! What the kernel needs from a global environment, and nothing more.
//!
//! `vf-kernel` deliberately knows nothing about *how* declarations are
//! registered, what axiom policy is in force, or how dependency
//! closures are tracked -- that is `vf-axioms`'/`vf-proof`'s job (a
//! later crate in verification-forge's implementation order). All the
//! kernel needs to type-check a term is: what a constant's type is,
//! and (for constants with a body) what it unfolds to. [`KernelEnv`]
//! is exactly that, and nothing more -- keeping the kernel's own
//! dependency surface minimal is part of what keeps it auditable.
use vf_core::{Symbol, TermId};

/// The kernel's only window into the global environment. Implemented
/// by `vf-axioms`'s registry; implemented here only by [`EmptyEnv`],
/// for checking closed terms that reference no constants.
pub trait KernelEnv {
    /// The type of the constant `name`, or `None` if `name` is not
    /// declared. The kernel treats "not declared" and "declared but
    /// inaccessible" identically: either way, using the name is a
    /// [`crate::KernelError::UnknownConstant`].
    fn type_of_const(&self, name: Symbol) -> Option<TermId>;

    /// What the constant `name` unfolds to for delta reduction, or
    /// `None` if it has no unfolding (an axiom, or an opaque
    /// declaration). Distinct from [`KernelEnv::type_of_const`]
    /// returning `None`: a name can have a type but no definition.
    fn unfold(&self, name: Symbol) -> Option<TermId>;

    /// If `name` is a registered recursor, its reduction shape (see
    /// `vf_reducer::RecursorSpec`). Defaulted to `None` so an
    /// environment with no inductive types (like [`EmptyEnv`]) needs
    /// no changes; `vf-axioms`'s registry is the real override.
    fn recursor(&self, _name: Symbol) -> Option<&vf_reducer::RecursorSpec> {
        None
    }
}

/// A [`KernelEnv`] with no declared constants. Useful for checking
/// terms that only use `vf-core`'s built-in formers
/// (`Sort`/`Pi`/`Lam`/`App`/`Let`/`Eq`), and in this crate's own tests.
pub struct EmptyEnv;

impl KernelEnv for EmptyEnv {
    fn type_of_const(&self, _name: Symbol) -> Option<TermId> {
        None
    }
    fn unfold(&self, _name: Symbol) -> Option<TermId> {
        None
    }
}

/// Adapts a [`KernelEnv`] to `vf-reducer`'s [`vf_reducer::DeltaContext`]
/// so kernel code can call [`vf_reducer::whnf`]/[`vf_reducer::normalize`]
/// directly without `vf-reducer` needing to know this trait exists.
pub(crate) struct AsDeltaContext<'a, E: KernelEnv>(pub &'a E);

impl<E: KernelEnv> vf_reducer::DeltaContext for AsDeltaContext<'_, E> {
    fn unfold(&self, name: Symbol) -> Option<TermId> {
        self.0.unfold(name)
    }
    fn recursor(&self, name: Symbol) -> Option<&vf_reducer::RecursorSpec> {
        self.0.recursor(name)
    }
}
