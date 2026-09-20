// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

use std::fmt;
use vf_core::{ArenaError, Symbol};
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
    /// `declare_recursor` named a constructor that isn't a declared
    /// [`crate::Registry`] builtin -- either never declared, or
    /// declared as something else (an axiom, a definition, another
    /// recursor). Registering a recursor over a dangling or
    /// mismatched reference is never allowed.
    UnknownConstructor(Symbol),
    /// A term-construction step (e.g. in `vf-axioms::prelude`) hit an
    /// out-of-range `TermId`. This should be unreachable for any term
    /// built entirely from a single arena's own constructors, so
    /// seeing it in practice means a `TermId` from a *different*
    /// arena was mixed in -- reported, not assumed away.
    Arena(ArenaError),
}

impl From<KernelError> for RegistryError {
    fn from(e: KernelError) -> Self {
        RegistryError::Kernel(e)
    }
}

impl From<ArenaError> for RegistryError {
    fn from(e: ArenaError) -> Self {
        RegistryError::Arena(e)
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
            RegistryError::UnknownConstructor(_) => {
                write!(
                    f,
                    "recursor references a name that isn't a declared builtin constructor"
                )
            }
            RegistryError::Arena(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for RegistryError {}
