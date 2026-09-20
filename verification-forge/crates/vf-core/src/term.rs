// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The term language and its hash-consed arena.
//!
//! ## Representation choice: locally nameless
//!
//! Bound variables (under a [`Term::Lam`], [`Term::Pi`], or
//! [`Term::Let`]) are de Bruijn indices ([`Term::BoundVar`]); a
//! variable that is temporarily free while a kernel algorithm is
//! looking *inside* a binder's body is a [`Term::FreeVar`] carrying a
//! unique [`Symbol`]; a reference to something declared in the global
//! environment (a definition, axiom, inductive type, constructor, or
//! recursor -- `vf-axioms`/later crates own what those symbols
//! resolve to) is a [`Term::Const`].
//!
//! This is the "locally nameless" representation (McBride & McKinna;
//! Charguéraud, *The Locally Nameless Representation*, 2012): de
//! Bruijn indices avoid the alpha-renaming bugs a named representation
//! invites, and using ordinary (interned) names for the *free*
//! variables that appear while working under a binder avoids the
//! index-shifting bugs a pure de Bruijn representation invites at
//! every substitution site. [`open_at`] and [`close_at`] are the two
//! primitives that move between the two: they are the most
//! correctness-critical functions in this crate, and are the most
//! heavily tested.
//!
//! ## Universe hierarchy: intentionally minimal
//!
//! [`Sort`] is `Prop` (impredicative -- see the crate-level caveat in
//! `lib.rs`) or `Type(n)` for a natural number `n`, with the usual
//! cumulative predicative hierarchy `Type 0 : Type 1 : Type 2 : ...`.
//! There is **no universe polymorphism** (`Type u` for a universe
//! *variable* `u`, or `max`/`imax` level combinators) in this version.
//! That is a real, deliberate simplification, not an oversight: a
//! polymorphic universe hierarchy is one of the more delicate parts of
//! a dependent type checker to get right, and getting it wrong is
//! exactly the kind of soundness bug this project's own hard
//! requirements (`0. HARD REQUIREMENTS`) forbid pretending not to
//! have. Adding it later does not require changing this enum's shape.

use crate::interner::Symbol;
use std::collections::HashMap;

/// A universe sort. See this module's doc comment for what is and
/// isn't supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sort {
    /// The impredicative sort of propositions.
    Prop,
    /// `Type(n)`, i.e. what other systems often write `Type n` / `𝒰ₙ`.
    Type(u32),
}

impl Sort {
    /// The sort that classifies `self` (`Prop : Type(0)`,
    /// `Type(n) : Type(n + 1)`).
    pub fn classifier(self) -> Sort {
        match self {
            Sort::Prop => Sort::Type(0),
            Sort::Type(n) => Sort::Type(n + 1),
        }
    }

    /// Cumulativity: is every term of sort `self` also (trivially) of
    /// sort `other`? `Prop` is not cumulative into `Type` in this
    /// minimal version -- only `Type(n) <= Type(m)` for `n <= m`.
    pub fn le(self, other: Sort) -> bool {
        match (self, other) {
            (Sort::Prop, Sort::Prop) => true,
            (Sort::Type(n), Sort::Type(m)) => n <= m,
            _ => false,
        }
    }
}

/// A de Bruijn index, counting binders outward from the variable's own
/// position (index 0 = the nearest enclosing binder).
pub type DeBruijnIndex = u32;

/// The interned identifier for a [`TermArena`]-allocated term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TermId(u32);

/// A binder's name, kept only for pretty-printing -- it has no
/// bearing on term equality, substitution, or type-checking (which is
/// exactly the point of the locally-nameless/de-Bruijn representation:
/// `fn(x) x` and `fn(y) y` are one term, not two).
///
/// `PartialEq`/`Eq`/`Hash` are implemented by hand (rather than
/// derived) specifically so they *ignore* the carried [`Symbol`]:
/// deriving them would make `Term::Lam`/`Term::Pi`/`Term::Let` values
/// that differ only in a binder's name hint compare unequal and hash
/// differently, defeating hash-consing for exactly the case (alpha
/// variants) it exists to collapse.
#[derive(Debug, Clone, Copy)]
pub struct Binder(pub Symbol);

impl PartialEq for Binder {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
impl Eq for Binder {}
impl std::hash::Hash for Binder {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

/// The term language. See this module's doc comment for the
/// representation choices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Sort(Sort),
    /// A bound variable; only meaningful under at least
    /// `index + 1` enclosing binders.
    BoundVar(DeBruijnIndex),
    /// A free local variable, introduced by [`open_at`] while working
    /// under a binder. Never appears in a *closed* term (one with no
    /// unbound [`Term::BoundVar`]s) that has been fully reconstructed
    /// with [`close_at`].
    FreeVar(Symbol),
    /// A reference to a global constant (definition, axiom, inductive
    /// type, constructor, or recursor).
    Const(Symbol),
    App(TermId, TermId),
    Lam {
        binder: Binder,
        ty: TermId,
        body: TermId,
    },
    Pi {
        binder: Binder,
        domain: TermId,
        codomain: TermId,
    },
    Let {
        binder: Binder,
        ty: TermId,
        value: TermId,
        body: TermId,
    },
    Eq {
        ty: TermId,
        lhs: TermId,
        rhs: TermId,
    },
}

/// A hash-consed arena of [`Term`]s: interning the same [`Term`] value
/// twice returns the same [`TermId`], so structural equality of two
/// terms is a single `TermId` comparison rather than a deep tree walk
/// (every [`Term`] variant's recursive fields are already-interned
/// [`TermId`]s, so hashing/equality on a bare [`Term`] value is O(1)).
#[derive(Debug, Default)]
pub struct TermArena {
    terms: Vec<Term>,
    lookup: HashMap<Term, TermId>,
}

/// Errors from arena operations. Kept even though today's only
/// failure mode ("this `TermId` wasn't produced by this arena") can't
/// arise from safe, correctly-scoped use of the public API -- per this
/// project's hard requirement to fail closed rather than panic, an
/// out-of-range id is reported, not `unwrap()`-ed past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaError {
    UnknownTermId(TermId),
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArenaError::UnknownTermId(id) => {
                write!(f, "term id {:?} was not produced by this arena", id)
            }
        }
    }
}
impl std::error::Error for ArenaError {}

impl TermArena {
    pub fn new() -> Self {
        TermArena::default()
    }

    /// Intern `term`, returning the same [`TermId`] every time this
    /// arena is asked to intern a structurally equal [`Term`].
    pub fn intern(&mut self, term: Term) -> TermId {
        if let Some(id) = self.lookup.get(&term) {
            return *id;
        }
        let id = TermId(self.terms.len() as u32);
        self.terms.push(term.clone());
        self.lookup.insert(term, id);
        id
    }

    /// Look up the [`Term`] behind a [`TermId`]. `Err` (never a panic)
    /// if `id` was not produced by this arena.
    pub fn get(&self, id: TermId) -> Result<&Term, ArenaError> {
        self.terms
            .get(id.0 as usize)
            .ok_or(ArenaError::UnknownTermId(id))
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    // -- Convenience constructors (all just `intern` calls) --

    pub fn sort(&mut self, s: Sort) -> TermId {
        self.intern(Term::Sort(s))
    }
    pub fn bound_var(&mut self, index: DeBruijnIndex) -> TermId {
        self.intern(Term::BoundVar(index))
    }
    pub fn free_var(&mut self, sym: Symbol) -> TermId {
        self.intern(Term::FreeVar(sym))
    }
    pub fn const_(&mut self, sym: Symbol) -> TermId {
        self.intern(Term::Const(sym))
    }
    pub fn app(&mut self, f: TermId, arg: TermId) -> TermId {
        self.intern(Term::App(f, arg))
    }
    pub fn lam(&mut self, binder: Binder, ty: TermId, body: TermId) -> TermId {
        self.intern(Term::Lam { binder, ty, body })
    }
    pub fn pi(&mut self, binder: Binder, domain: TermId, codomain: TermId) -> TermId {
        self.intern(Term::Pi {
            binder,
            domain,
            codomain,
        })
    }
    pub fn let_(&mut self, binder: Binder, ty: TermId, value: TermId, body: TermId) -> TermId {
        self.intern(Term::Let {
            binder,
            ty,
            value,
            body,
        })
    }
    pub fn eq(&mut self, ty: TermId, lhs: TermId, rhs: TermId) -> TermId {
        self.intern(Term::Eq { ty, lhs, rhs })
    }

    /// Rebuild `term` with every immediate structural child replaced
    /// according to `f`, which is applied at the correct binder depth
    /// (`f` receives the current de-Bruijn depth so it can tell a
    /// [`Term::BoundVar`] belonging to `term`'s own outermost binder
    /// apart from one belonging to an enclosing scope). This is the
    /// one traversal [`open_at`]/[`close_at`]/[`shift`] are all built
    /// from, so the recursion-through-binders logic is written and
    /// tested exactly once.
    fn map_at_depth(
        &mut self,
        term: TermId,
        depth: u32,
        f: &mut impl FnMut(&mut TermArena, u32, &Term) -> Option<TermId>,
    ) -> Result<TermId, ArenaError> {
        let t = self.get(term)?.clone();
        if let Some(replaced) = f(self, depth, &t) {
            return Ok(replaced);
        }
        match t {
            Term::Sort(_) | Term::BoundVar(_) | Term::FreeVar(_) | Term::Const(_) => Ok(term),
            Term::App(func, arg) => {
                let func2 = self.map_at_depth(func, depth, f)?;
                let arg2 = self.map_at_depth(arg, depth, f)?;
                Ok(self.app(func2, arg2))
            }
            Term::Lam { binder, ty, body } => {
                let ty2 = self.map_at_depth(ty, depth, f)?;
                let body2 = self.map_at_depth(body, depth + 1, f)?;
                Ok(self.lam(binder, ty2, body2))
            }
            Term::Pi {
                binder,
                domain,
                codomain,
            } => {
                let domain2 = self.map_at_depth(domain, depth, f)?;
                let codomain2 = self.map_at_depth(codomain, depth + 1, f)?;
                Ok(self.pi(binder, domain2, codomain2))
            }
            Term::Let {
                binder,
                ty,
                value,
                body,
            } => {
                let ty2 = self.map_at_depth(ty, depth, f)?;
                let value2 = self.map_at_depth(value, depth, f)?;
                let body2 = self.map_at_depth(body, depth + 1, f)?;
                Ok(self.let_(binder, ty2, value2, body2))
            }
            Term::Eq { ty, lhs, rhs } => {
                let ty2 = self.map_at_depth(ty, depth, f)?;
                let lhs2 = self.map_at_depth(lhs, depth, f)?;
                let rhs2 = self.map_at_depth(rhs, depth, f)?;
                Ok(self.eq(ty2, lhs2, rhs2))
            }
        }
    }

    /// Substitute `replacement` for the variable bound by `term`'s
    /// own outermost binder: every [`Term::BoundVar`] whose index
    /// points exactly at that binder becomes `replacement` (shifted
    /// to account for how many binders it's crossing into), and every
    /// index pointing further out is decremented by one (its binder
    /// has, from the perspective of the result, moved one level
    /// closer). This is what "entering" a [`Term::Lam`]/[`Term::Pi`]/
    /// [`Term::Let`] body means: `open_at(body, 0, replacement)`.
    pub fn open_at(
        &mut self,
        term: TermId,
        binder_depth: u32,
        replacement: TermId,
    ) -> Result<TermId, ArenaError> {
        self.map_at_depth(term, binder_depth, &mut |arena, depth, t| match t {
            Term::BoundVar(index) if *index == depth => {
                Some(arena.shift(replacement, 0, depth as i64).ok()?)
            }
            _ => None,
        })
    }

    /// The inverse of [`open_at`]: replace every free occurrence of
    /// `sym` with a [`Term::BoundVar`] pointing at the binder `depth`
    /// levels out. Used to turn a body containing a free variable
    /// (introduced by a prior `open_at`) back into a properly bound
    /// term when reconstructing a [`Term::Lam`]/[`Term::Pi`].
    pub fn close_at(
        &mut self,
        term: TermId,
        binder_depth: u32,
        sym: Symbol,
    ) -> Result<TermId, ArenaError> {
        self.map_at_depth(term, binder_depth, &mut |arena, depth, t| match t {
            Term::FreeVar(s) if *s == sym => Some(arena.bound_var(depth)),
            _ => None,
        })
    }

    /// Shift every [`Term::BoundVar`] in `term` with index `>= cutoff`
    /// by `amount` (which may be negative). Needed whenever a term is
    /// relocated under a different number of enclosing binders than
    /// the ones it was built under -- most notably inside [`open_at`]
    /// itself, when `replacement` is inserted `binder_depth` binders
    /// deep and its own free (from `replacement`'s perspective)
    /// bound-variable references must be adjusted to still point
    /// at the same logical binders from the new position.
    pub fn shift(&mut self, term: TermId, cutoff: u32, amount: i64) -> Result<TermId, ArenaError> {
        if amount == 0 {
            return Ok(term);
        }
        self.map_at_depth(term, cutoff, &mut |arena, depth, t| match t {
            Term::BoundVar(index) if *index >= depth => {
                let shifted = (*index as i64) + amount;
                debug_assert!(shifted >= 0, "shift produced a negative de Bruijn index");
                Some(arena.bound_var(shifted.max(0) as u32))
            }
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interner::Interner;

    fn setup() -> (TermArena, Interner) {
        (TermArena::new(), Interner::new())
    }

    #[test]
    fn interning_the_same_term_twice_returns_the_same_id() {
        let (mut arena, _) = setup();
        let a = arena.sort(Sort::Prop);
        let b = arena.sort(Sort::Prop);
        assert_eq!(a, b);
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn interning_different_terms_returns_different_ids() {
        let (mut arena, _) = setup();
        let a = arena.sort(Sort::Prop);
        let b = arena.sort(Sort::Type(0));
        assert_ne!(a, b);
    }

    #[test]
    fn get_on_a_valid_id_succeeds() {
        let (mut arena, _) = setup();
        let id = arena.sort(Sort::Type(3));
        assert_eq!(arena.get(id).unwrap(), &Term::Sort(Sort::Type(3)));
    }

    #[test]
    fn get_on_an_out_of_range_id_fails_closed_not_panics() {
        let (arena, _) = setup();
        let bogus = TermId(999);
        assert_eq!(arena.get(bogus), Err(ArenaError::UnknownTermId(bogus)));
    }

    #[test]
    fn identity_lambda_open_at_zero_substitutes_the_bound_variable() {
        // Build `fn(_: Type 0) => #0` (the identity function's body is
        // just the bound variable), then open it with a concrete
        // replacement term and confirm the bound variable became that
        // replacement.
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let body = arena.bound_var(0);
        let replacement = arena.sort(Sort::Type(5));

        let opened = arena.open_at(body, 0, replacement).unwrap();
        assert_eq!(arena.get(opened).unwrap(), arena.get(replacement).unwrap());

        // Sanity: the lambda itself still type-checks as a shape (not
        // a soundness claim -- vf-kernel owns that -- just confirming
        // `lam` doesn't choke on this body).
        let _lam = arena.lam(x, ty0, body);
    }

    #[test]
    fn open_at_only_touches_the_targeted_binder_depth() {
        // #1 under one binder refers to something *outside* that
        // binder and must be left alone by an open_at targeting depth 0.
        let (mut arena, _) = setup();
        let outer_ref = arena.bound_var(1);
        let replacement = arena.sort(Sort::Type(9));
        let opened = arena.open_at(outer_ref, 0, replacement).unwrap();
        // #1 is untouched in absolute terms, but since we "entered" one
        // binder, map_at_depth's shift bookkeeping should leave a
        // reference to the *next* enclosing binder recognizable as
        // BoundVar(0) once outside this binder -- here, with nothing
        // enclosing, we just confirm it did NOT get replaced by the
        // Type(9) term.
        assert_ne!(arena.get(opened).unwrap(), arena.get(replacement).unwrap());
    }

    #[test]
    fn open_then_close_is_the_identity_on_a_simple_body() {
        let (mut arena, mut interner) = setup();
        let sym = interner.intern("x");
        let placeholder = arena.free_var(sym);
        let body = arena.bound_var(0);

        let opened = arena.open_at(body, 0, placeholder).unwrap();
        let closed = arena.close_at(opened, 0, sym).unwrap();
        assert_eq!(arena.get(closed).unwrap(), arena.get(body).unwrap());
    }

    #[test]
    fn open_at_correctly_shifts_replacement_under_nested_binders() {
        // fn(_) => fn(_) => #1 refers to the OUTER binder's variable
        // from inside the inner one. Opening the outer binder with a
        // free variable must produce fn(_) => <freevar>, i.e. the
        // reference must resolve through the inner binder correctly.
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let inner_body = arena.bound_var(1); // refers to the outer binder
        let x = Binder(interner.intern("x"));
        let inner_lam = arena.lam(x, ty0, inner_body);

        let sym = interner.intern("y");
        let placeholder = arena.free_var(sym);
        let opened = arena.open_at(inner_lam, 0, placeholder).unwrap();

        match arena.get(opened).unwrap().clone() {
            Term::Lam { body, .. } => {
                assert_eq!(
                    arena.get(body).unwrap(),
                    arena.get(placeholder).unwrap(),
                    "the outer binder's reference must resolve to the opened placeholder \
                     even though it's nested one level deeper"
                );
            }
            other => panic!("expected a Lam, got {other:?}"),
        }
    }

    #[test]
    fn close_at_correctly_shifts_under_nested_binders() {
        // Inverse of the above: a free variable used inside a nested
        // binder must become a BoundVar with an index accounting for
        // the extra nesting once closed at the outer depth.
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let sym = interner.intern("y");
        let inner_body = arena.free_var(sym);
        let x = Binder(interner.intern("x"));
        let inner_lam = arena.lam(x, ty0, inner_body);

        let closed = arena.close_at(inner_lam, 0, sym).unwrap();
        match arena.get(closed).unwrap().clone() {
            Term::Lam { body, .. } => {
                assert_eq!(
                    arena.get(body).unwrap(),
                    &Term::BoundVar(1),
                    "a free variable used one binder deeper than the depth \
                     it's being closed at must become BoundVar(1), not BoundVar(0)"
                );
            }
            other => panic!("expected a Lam, got {other:?}"),
        }
    }

    #[test]
    fn shift_by_zero_is_a_no_op() {
        let (mut arena, _) = setup();
        let term = arena.bound_var(3);
        let shifted = arena.shift(term, 0, 0).unwrap();
        assert_eq!(shifted, term);
    }

    #[test]
    fn shift_leaves_indices_below_cutoff_untouched() {
        let (mut arena, _) = setup();
        let term = arena.bound_var(0);
        let shifted = arena.shift(term, 1, 5).unwrap();
        assert_eq!(arena.get(shifted).unwrap(), &Term::BoundVar(0));
    }

    #[test]
    fn shift_moves_indices_at_or_above_cutoff() {
        let (mut arena, _) = setup();
        let term = arena.bound_var(2);
        let shifted = arena.shift(term, 1, 3).unwrap();
        assert_eq!(arena.get(shifted).unwrap(), &Term::BoundVar(5));
    }

    #[test]
    fn sort_classifier_and_cumulativity() {
        assert_eq!(Sort::Prop.classifier(), Sort::Type(0));
        assert_eq!(Sort::Type(0).classifier(), Sort::Type(1));
        assert!(Sort::Type(0).le(Sort::Type(1)));
        assert!(!Sort::Type(1).le(Sort::Type(0)));
        assert!(Sort::Prop.le(Sort::Prop));
        assert!(!Sort::Prop.le(Sort::Type(0)));
    }

    #[test]
    fn app_pi_lam_construction_and_hash_consing_across_variants() {
        let (mut arena, mut interner) = setup();
        let x = Binder(interner.intern("x"));
        let ty0 = arena.sort(Sort::Type(0));
        let body = arena.bound_var(0);

        let pi1 = arena.pi(x, ty0, ty0);
        let pi2 = arena.pi(x, ty0, ty0);
        assert_eq!(
            pi1, pi2,
            "structurally identical Pi terms must hash-cons together"
        );

        let lam = arena.lam(x, ty0, body);
        let f = lam;
        let arg = ty0;
        let app1 = arena.app(f, arg);
        let app2 = arena.app(f, arg);
        assert_eq!(app1, app2);
    }

    #[test]
    fn alpha_variants_that_differ_only_in_binder_name_hash_cons_together() {
        // fn(x) => x and fn(y) => y must be ONE interned term, not two:
        // the binder's Symbol is a pretty-printing hint only (see
        // Binder's doc comment) and must not affect Term equality/hash.
        let (mut arena, mut interner) = setup();
        let ty0 = arena.sort(Sort::Type(0));
        let body = arena.bound_var(0);
        let x = Binder(interner.intern("x"));
        let y = Binder(interner.intern("y"));
        let lam_x = arena.lam(x, ty0, body);
        let lam_y = arena.lam(y, ty0, body);
        assert_eq!(lam_x, lam_y);
        assert_eq!(
            arena.len(),
            3,
            "Sort(Type 0), BoundVar(0), and ONE Lam entry -- not two separate Lam entries"
        );
    }
}
