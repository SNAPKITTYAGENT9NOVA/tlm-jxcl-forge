// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Core term representation for verification-forge's dependent type
//! kernel.
//!
//! This crate is the bottom of the whole verification-forge stack: it
//! depends on nothing else in the workspace, and everything else
//! (`vf-typecheck`'s elaborator, `vf-reducer`'s reduction engine,
//! `vf-kernel`'s trusted checker) is built on the [`Term`]/[`TermId`]/
//! [`TermArena`] representation defined here.
//!
//! **This crate does not itself decide whether anything is
//! well-typed, well-formed, or true.** It only defines the shape of
//! terms and the structural operations ([`open_at`](TermArena::open_at),
//! [`close_at`](TermArena::close_at), [`shift`](TermArena::shift))
//! needed to build and take apart binders correctly. See
//! `vf-kernel`'s crate docs and `docs/TRUST_MODEL.md` for where actual
//! typing/proof-checking judgments live.
//!
//! ## Known, documented simplifications (not bugs)
//!
//! - No universe polymorphism: [`Sort`] is `Prop` or `Type(n)` for a
//!   concrete `n: u32`, never a universe *variable*. See [`term`]'s
//!   module docs.
//! - No proof irrelevance: two [`Term`]s of sort `Prop` that are
//!   propositionally equal are not automatically definitionally equal
//!   just because they inhabit `Prop`. `vf-kernel`'s `definitional_equal`
//!   treats `Prop` exactly like any other sort for this purpose.
//!
//! Both are real restrictions on what can be expressed/proved, listed
//! here (and in `docs/TRUST_MODEL.md`) precisely so nothing downstream
//! can quietly assume more power than this kernel actually has.

pub mod interner;
pub mod term;

pub use interner::{Interner, Symbol};
pub use term::{ArenaError, Binder, DeBruijnIndex, Sort, Term, TermArena, TermId};
