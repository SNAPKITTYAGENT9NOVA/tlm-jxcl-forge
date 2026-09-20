// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A checked theorem's identity: its own declared name, paired with
//! its statement, its proof term, and the dependency closure that
//! backs it.
use crate::closure::{dependency_closure, DependencyClosure};
use crate::error::ProofError;
use vf_axioms::Registry;
use vf_core::{Symbol, TermArena, TermId};

/// Every theorem/proof in verification-forge has explicit identity:
/// this struct *is* that identity, not just a bag of terms. `name` is
/// the [`vf_axioms::Registry`] entry this was checked as -- there is
/// no such thing as an anonymous or ad hoc `CheckedTheorem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedTheorem {
    pub name: Symbol,
    pub statement: TermId,
    pub proof: TermId,
    pub dependencies: DependencyClosure,
}

impl CheckedTheorem {
    /// Whether this theorem's truth rests on the kernel alone, with no
    /// axioms assumed anywhere in its transitive closure.
    pub fn is_axiom_free(&self) -> bool {
        self.dependencies.is_axiom_free()
    }
}

/// Look up `name` in `registry` (which must already have accepted it
/// via `declare_definition`/`declare_theorem` -- this function does no
/// checking of its own, it only assembles what the registry already
/// established) and compute its [`CheckedTheorem`] record.
pub fn checked_theorem(
    arena: &TermArena,
    registry: &Registry,
    name: Symbol,
) -> Result<CheckedTheorem, ProofError> {
    if !registry.is_declared(name) {
        return Err(ProofError::UndeclaredRoot(name));
    }
    if registry.is_axiom(name) {
        // An axiom has no proof term to report -- asking for one here
        // is a caller error, not something to paper over by fabricating
        // a proof or silently treating the axiom itself as its own proof.
        return Err(ProofError::NotATheorem(name));
    }
    let statement = registry
        .type_of(name)
        .ok_or(ProofError::UnknownConstant(name))?;
    let proof = registry
        .value_of(name)
        .ok_or(ProofError::UnknownConstant(name))?;
    let dependencies = dependency_closure(arena, registry, name)?;
    Ok(CheckedTheorem {
        name,
        statement,
        proof,
        dependencies,
    })
}
