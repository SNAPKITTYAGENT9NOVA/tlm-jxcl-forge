//! Transitive axiom/definition dependency closures.
//!
//! Every declaration a [`vf_axioms::Registry`] accepts can only
//! reference names declared *before* it (the registry checks a new
//! declaration against itself as it stood beforehand, never including
//! the declaration under construction) -- so the reference graph over
//! declared names is a DAG. That's what makes the plain recursive walk
//! here safe without separate cycle detection: a cycle would require a
//! forward reference, and the registry can't produce one.
use crate::error::ProofError;
use std::collections::BTreeSet;
use vf_axioms::Registry;
use vf_core::{Symbol, Term, TermArena, TermId};

/// Every symbol `Term::Const` refers to, directly, within `term` --
/// not transitively resolved through the registry yet.
fn collect_direct_consts(
    arena: &TermArena,
    term: TermId,
    out: &mut BTreeSet<Symbol>,
) -> Result<(), ProofError> {
    let t = arena.get(term)?.clone();
    match t {
        Term::Sort(_) | Term::BoundVar(_) | Term::FreeVar(_) => {}
        Term::Const(sym) => {
            out.insert(sym);
        }
        Term::App(f, a) => {
            collect_direct_consts(arena, f, out)?;
            collect_direct_consts(arena, a, out)?;
        }
        Term::Lam { ty, body, .. } => {
            collect_direct_consts(arena, ty, out)?;
            collect_direct_consts(arena, body, out)?;
        }
        Term::Pi {
            domain, codomain, ..
        } => {
            collect_direct_consts(arena, domain, out)?;
            collect_direct_consts(arena, codomain, out)?;
        }
        Term::Let {
            ty, value, body, ..
        } => {
            collect_direct_consts(arena, ty, out)?;
            collect_direct_consts(arena, value, out)?;
            collect_direct_consts(arena, body, out)?;
        }
        Term::Eq { ty, lhs, rhs } => {
            collect_direct_consts(arena, ty, out)?;
            collect_direct_consts(arena, lhs, out)?;
            collect_direct_consts(arena, rhs, out)?;
        }
    }
    Ok(())
}

/// A declared name's full transitive dependency set, split by what
/// kind of declaration each dependency is. `BTreeSet` (rather than a
/// hash set) so results are deterministically ordered -- this
/// project's hard requirement of deterministic output extends to
/// diagnostic/audit data like this, not just proof verdicts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyClosure {
    pub axioms: BTreeSet<Symbol>,
    pub definitions: BTreeSet<Symbol>,
}

impl DependencyClosure {
    /// A declaration is only "purely kernel-verified, no assumptions
    /// beyond the kernel itself" if its closure contains zero axioms.
    /// Any non-empty [`DependencyClosure::axioms`] means the
    /// declaration's truth is conditional on those axioms holding --
    /// this is the fact `vf-ledger`'s KERNEL_VERIFIED-vs-AXIOM_DEPENDENT
    /// status decision (a later crate) is ultimately built on.
    pub fn is_axiom_free(&self) -> bool {
        self.axioms.is_empty()
    }
}

/// Compute `root`'s transitive dependency closure: every axiom and
/// definition its type and value (transitively, through every
/// definition it references) depend on. If `root` is itself declared
/// as an axiom, its closure is trivially `{axioms: {root}}` -- using
/// an axiom's own name in a closure computation is exactly what
/// "assuming this axiom" means.
pub fn dependency_closure(
    arena: &TermArena,
    registry: &Registry,
    root: Symbol,
) -> Result<DependencyClosure, ProofError> {
    if !registry.is_declared(root) {
        return Err(ProofError::UndeclaredRoot(root));
    }

    let mut closure = DependencyClosure::default();
    if registry.is_axiom(root) {
        closure.axioms.insert(root);
        return Ok(closure);
    }

    let mut visited_defs: BTreeSet<Symbol> = BTreeSet::new();
    let mut frontier: Vec<Symbol> = vec![root];
    while let Some(name) = frontier.pop() {
        if registry.is_axiom(name) {
            closure.axioms.insert(name);
            continue; // an axiom has no body to recurse into
        }
        if !visited_defs.insert(name) {
            continue; // already expanded this definition
        }
        if name != root {
            closure.definitions.insert(name);
        }

        let ty = registry
            .type_of(name)
            .ok_or(ProofError::UnknownConstant(name))?;
        let value = registry
            .value_of(name)
            .ok_or(ProofError::UnknownConstant(name))?;
        let mut direct = BTreeSet::new();
        collect_direct_consts(arena, ty, &mut direct)?;
        collect_direct_consts(arena, value, &mut direct)?;
        for dep in direct {
            if !registry.is_declared(dep) {
                return Err(ProofError::UnknownConstant(dep));
            }
            frontier.push(dep);
        }
    }
    Ok(closure)
}
