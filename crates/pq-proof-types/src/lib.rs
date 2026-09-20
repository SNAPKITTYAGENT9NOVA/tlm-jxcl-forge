// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Backend-independent proof types: the [`ProofScheme`] trait and the
//! shared [`AttestationBytes`]/[`ProofError`] types, so callers never
//! depend on a concrete proof backend (e.g. arkworks) directly.
//!
//! # Why `verify` takes bytes, not an associated type
//!
//! An earlier draft of this trait carried an associated `Attestation`
//! type (`fn verify(&self, context: &[u8], attestation: &Self::Attestation)`).
//! That shape is natural for a single concrete backend, but
//! `pq-proof-registry` exists specifically to hold *heterogeneous*
//! schemes -- Groth16-Pedersen today, something else tomorrow -- behind
//! one `HashMap<String, Box<dyn ProofScheme>>`. A trait with an
//! associated type used in a method signature is not object-safe (Rust
//! can't build a vtable for `Box<dyn ProofScheme>` without knowing which
//! concrete `Attestation` type each entry uses), so it cannot be stored
//! that way.
//!
//! [`ProofScheme::verify`] therefore takes an opaque [`AttestationBytes`]
//! instead: every concrete scheme decodes its own bytes internally (as
//! `pq-error-proof`'s impl does via its existing `Attestation::from_bytes`)
//! before running its real verification logic. This keeps the trait
//! object-safe, keeps arkworks (or any other backend's) types out of
//! this crate entirely, and is what lets `pq-proof-registry` and
//! `pq-proof-verifier` hold/call schemes without ever importing a
//! concrete backend crate.
//!
//! Similarly, `scheme_id` takes `&self` rather than being a bare
//! associated function (`fn scheme_id() -> &'static str`), again so it
//! is callable through a trait object.
//!
//! Owns: The [`ProofScheme`] trait.
#![forbid(unsafe_code)]

use std::fmt;

/// Opaque, serialized attestation bytes for a [`ProofScheme`]. Callers
/// that only need to move an attestation around (store it, transmit it,
/// hand it to [`ProofScheme::verify`]) never need to know the concrete
/// attestation type a given scheme uses internally.
///
/// A scheme's own `Attestation::to_bytes`/`from_bytes` (or equivalent)
/// is what produces/consumes this; this type is just a labeled carrier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct AttestationBytes(pub Vec<u8>);

impl AttestationBytes {
    /// Wrap already-serialized attestation bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        AttestationBytes(bytes)
    }

    /// Borrow the underlying bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Consume this wrapper, returning the underlying bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for AttestationBytes {
    fn from(bytes: Vec<u8>) -> Self {
        AttestationBytes(bytes)
    }
}

impl AsRef<[u8]> for AttestationBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Errors a [`ProofScheme`] can report. Distinct from that scheme's own
/// backend-specific error type (e.g. `pq_error_proof::Error`): this is
/// the backend-independent shape every scheme's errors get mapped into
/// at the trait boundary, carrying the original error's `Display` text
/// for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofError {
    /// The supplied [`AttestationBytes`] could not be decoded into this
    /// scheme's concrete attestation type (truncated, corrupt, or from
    /// a different scheme entirely).
    Decoding(String),
    /// Verification itself could not be completed (e.g. a malformed
    /// proof triggered a backend error). Distinct from a well-formed
    /// attestation that simply does not hold -- that case is `Ok(false)`
    /// from [`ProofScheme::verify`], not an `Err`.
    Verification(String),
    /// No scheme is registered under the requested scheme id.
    UnknownScheme(String),
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofError::Decoding(msg) => write!(f, "failed to decode attestation: {msg}"),
            ProofError::Verification(msg) => write!(f, "proof verification error: {msg}"),
            ProofError::UnknownScheme(id) => write!(f, "no proof scheme registered as {id:?}"),
        }
    }
}

impl std::error::Error for ProofError {}

/// A pluggable proof scheme: something that can verify an opaque
/// attestation against a public context under some scheme-specific
/// public parameters (a verifying key, a Pedersen generator pair,
/// etc. -- kept entirely inside the implementor).
///
/// Implementors typically wrap an existing "params" type that already
/// knows how to verify (e.g. `pq-error-proof`'s `Params`) with minimal
/// glue: decode [`AttestationBytes`] into the concrete attestation type
/// and delegate to the existing, unchanged verification logic.
///
/// Object-safe by construction (see the module docs for why), so it can
/// be stored as `Box<dyn ProofScheme>` in a registry alongside other,
/// unrelated schemes.
pub trait ProofScheme {
    /// A stable identifier for this scheme (e.g.
    /// `"pq-error-proof/groth16-pedersen-commitment-opening/v1"`), used
    /// as the registry lookup key. Stable across calls for a given
    /// instance/backend version.
    fn scheme_id(&self) -> &'static str;

    /// Verify `attestation` against `context` under this scheme's own
    /// (internally held) public parameters.
    ///
    /// `Ok(false)` means the attestation was well-formed but does not
    /// hold (wrong context, wrong commitment, forged proof -- verified
    /// closed, not erroneously). `Err` means verification could not be
    /// completed at all (undecodable bytes, a malformed proof that
    /// errors rather than fails the backend's pairing/consistency
    /// check).
    fn verify(&self, context: &[u8], attestation: &AttestationBytes) -> Result<bool, ProofError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial in-crate test double: "verifies" iff the attestation
    /// bytes equal a fixed accepted context transformed by `!` (bitwise
    /// negation) byte-for-byte, purely to exercise the trait plumbing
    /// without any real cryptography.
    struct FlipScheme;

    impl ProofScheme for FlipScheme {
        fn scheme_id(&self) -> &'static str {
            "test/flip/v1"
        }

        fn verify(
            &self,
            context: &[u8],
            attestation: &AttestationBytes,
        ) -> Result<bool, ProofError> {
            if attestation.as_slice().len() < context.len() {
                return Err(ProofError::Decoding(
                    "attestation shorter than context".into(),
                ));
            }
            let flipped: Vec<u8> = context.iter().map(|b| !b).collect();
            Ok(attestation.as_slice() == flipped.as_slice())
        }
    }

    #[test]
    fn attestation_bytes_roundtrip_conversions() {
        let raw = vec![1u8, 2, 3, 4];
        let wrapped: AttestationBytes = raw.clone().into();
        assert_eq!(wrapped.as_slice(), raw.as_slice());
        assert_eq!(wrapped.as_ref(), raw.as_slice());
        assert_eq!(wrapped.into_vec(), raw);
    }

    #[test]
    fn attestation_bytes_default_is_empty() {
        assert_eq!(AttestationBytes::default().as_slice(), &[] as &[u8]);
    }

    #[test]
    fn proof_error_display_messages_are_informative() {
        assert!(ProofError::Decoding("bad".into())
            .to_string()
            .contains("bad"));
        assert!(ProofError::Verification("nope".into())
            .to_string()
            .contains("nope"));
        assert!(ProofError::UnknownScheme("foo".into())
            .to_string()
            .contains("foo"));
    }

    #[test]
    fn scheme_object_safety_and_verify_accept() {
        let scheme: Box<dyn ProofScheme> = Box::new(FlipScheme);
        let context = b"hello";
        let attestation = AttestationBytes::new(context.iter().map(|b| !b).collect());

        assert_eq!(scheme.scheme_id(), "test/flip/v1");
        assert!(scheme.verify(context, &attestation).unwrap());
    }

    #[test]
    fn scheme_verify_reject_for_wrong_attestation() {
        let scheme: Box<dyn ProofScheme> = Box::new(FlipScheme);
        let context = b"hello";
        let attestation = AttestationBytes::new(b"wrong".to_vec());

        assert!(!scheme.verify(context, &attestation).unwrap());
    }

    #[test]
    fn scheme_verify_errors_on_undecodable_bytes() {
        let scheme: Box<dyn ProofScheme> = Box::new(FlipScheme);
        let context = b"hello there";
        let attestation = AttestationBytes::new(b"short".to_vec());

        assert!(matches!(
            scheme.verify(context, &attestation),
            Err(ProofError::Decoding(_))
        ));
    }
}
