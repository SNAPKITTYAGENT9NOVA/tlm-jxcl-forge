//! The trusted proof-checking kernel.
//!
//! ## Trust status: this crate IS the trusted computing base
//!
//! Per `docs/TRUST_MODEL.md`, verification-forge's trusted computing
//! base is only this kernel, the proof-object validator, and the
//! crypto hash implementation -- nothing else. `vf-lexer`,
//! `vf-parser`, `vf-elab`, and `vf-typecheck` are all *evidence
//! generators*: whatever candidate term or claimed type they produce
//! is only ever accepted after this crate independently re-derives
//! it. `vf-reducer` is trusted alongside this crate (the kernel has no
//! way to check its own reduction/equality primitive without an
//! infinite regress), so it is the one other crate this one depends
//! on.
//!
//! This crate never calls into `vf-typecheck`. See [`judgment`]'s
//! module doc for why that's a hard boundary, not an accident of
//! dependency ordering.
//!
//! ## What's here
//!
//! - [`env::KernelEnv`]: the minimal trait the kernel needs from a
//!   global environment (a constant's type, and what it unfolds to).
//!   `vf-axioms` (a later crate) implements it against a real,
//!   policy-enforcing axiom/definition registry; [`env::EmptyEnv`] is
//!   the trivial implementation used to check closed terms and in this
//!   crate's own tests.
//! - [`context::Context`]: the local typing context.
//! - [`judgment::infer_type`], [`judgment::check`],
//!   [`judgment::definitional_equal`]: the three judgments themselves.
//! - [`error::KernelError`]: everything a judgment can fail with.
#![forbid(unsafe_code)]

pub mod context;
pub mod env;
pub mod error;
pub mod judgment;

pub use context::Context;
pub use env::{EmptyEnv, KernelEnv};
pub use error::KernelError;
pub use judgment::{check, definitional_equal, infer_type};
