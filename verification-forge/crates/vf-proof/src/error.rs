// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

use std::fmt;
use vf_core::{ArenaError, Symbol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofError {
    Arena(ArenaError),
    /// `root` (or something in its transitive closure) references a
    /// name the registry has no record of at all. This should be
    /// unreachable for anything the registry itself accepted (see
    /// `vf-axioms::Registry`'s doc comment on why its reference graph
    /// is a DAG over already-declared names), so seeing it in practice
    /// would mean the `Registry` passed in isn't the one that actually
    /// checked `root` -- reported, not assumed away.
    UnknownConstant(Symbol),
    /// The requested root name was never declared in this registry at
    /// all.
    UndeclaredRoot(Symbol),
    /// [`crate::checked_theorem`] was asked for a name that's declared
    /// as an axiom, not a definition/theorem -- an axiom has no proof
    /// term to report, and this is never silently substituted with
    /// one.
    NotATheorem(Symbol),
}

impl From<ArenaError> for ProofError {
    fn from(e: ArenaError) -> Self {
        ProofError::Arena(e)
    }
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofError::Arena(e) => write!(f, "{e}"),
            ProofError::UnknownConstant(_) => {
                write!(
                    f,
                    "a dependency references a name absent from this registry"
                )
            }
            ProofError::UndeclaredRoot(_) => write!(f, "the requested name was never declared"),
            ProofError::NotATheorem(_) => write!(
                f,
                "the requested name is declared as an axiom, not a theorem"
            ),
        }
    }
}
impl std::error::Error for ProofError {}
