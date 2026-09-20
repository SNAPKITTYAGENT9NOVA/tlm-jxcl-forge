// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The explicit axiom/definition registry.
//!
//! ## Trust status: UNTRUSTED, but fail-closed
//!
//! This crate is not part of the trusted computing base (that's
//! `vf-kernel` + `vf-reducer`, per `docs/TRUST_MODEL.md`), but it is
//! the single gate every axiom, definition, and theorem in a
//! verification-forge session passes through: [`Registry`] rejects
//! (never silently drops or "fixes up") a duplicate name, a policy
//! violation, or anything the kernel doesn't independently confirm.
//! There is exactly one function that can introduce an axiom
//! ([`Registry::declare_axiom`]), it always requires a caller-supplied
//! justification string when [`AxiomPolicy`] calls for one, and
//! nothing in this crate ever adds an axiom as a side effect of
//! something else -- this is what "never silently introduce an axiom"
//! means in code.
#![forbid(unsafe_code)]

mod error;
mod policy;
mod prelude;
mod registry;

pub use error::RegistryError;
pub use policy::AxiomPolicy;
pub use prelude::{register_prelude, Prelude};
pub use registry::Registry;
