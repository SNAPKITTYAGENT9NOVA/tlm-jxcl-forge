// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A verifier facade that looks up the right scheme via
//! `pq-proof-registry` and verifies, so callers never touch a concrete
//! proof backend's types (e.g. arkworks) directly -- they only need a
//! [`ProofRegistry`], a scheme id string, a context byte slice, and an
//! [`AttestationBytes`].
//!
//! Owns: [`verify_attestation`].
#![forbid(unsafe_code)]

pub use pq_proof_registry::ProofRegistry;
pub use pq_proof_types::{AttestationBytes, ProofError};

/// Look up `scheme_id` in `registry` and verify `attestation` against
/// `context` under it.
///
/// Returns [`ProofError::UnknownScheme`] if no scheme is registered
/// under `scheme_id`; otherwise defers to that scheme's own
/// `ProofScheme::verify` (see that trait's docs for the meaning of
/// `Ok(false)` vs. `Err`).
pub fn verify_attestation(
    registry: &ProofRegistry,
    scheme_id: &str,
    context: &[u8],
    attestation: &AttestationBytes,
) -> Result<bool, ProofError> {
    let scheme = registry
        .get(scheme_id)
        .ok_or_else(|| ProofError::UnknownScheme(scheme_id.to_string()))?;
    scheme.verify(context, attestation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pq_proof_types::ProofScheme;

    /// Same fake-scheme test-double pattern as `pq-proof-registry`'s
    /// unit tests: a trivial local `ProofScheme`, not `pq-error-proof`,
    /// so this crate's tests stay fast and free of a real backend.
    struct FakeScheme {
        id: &'static str,
    }

    impl ProofScheme for FakeScheme {
        fn scheme_id(&self) -> &'static str {
            self.id
        }

        fn verify(
            &self,
            context: &[u8],
            attestation: &AttestationBytes,
        ) -> Result<bool, ProofError> {
            // "Verifies" iff the attestation bytes equal the context
            // bytes -- enough to exercise real accept/reject paths
            // without any real cryptography.
            Ok(attestation.as_slice() == context)
        }
    }

    fn registry_with_fake_scheme() -> ProofRegistry {
        let mut registry = ProofRegistry::new();
        registry.register("fake/v1", Box::new(FakeScheme { id: "fake/v1" }));
        registry
    }

    #[test]
    fn verify_attestation_accepts_a_matching_attestation() {
        let registry = registry_with_fake_scheme();
        let context = b"error_code=EXAMPLE";
        let attestation = AttestationBytes::new(context.to_vec());

        assert!(verify_attestation(&registry, "fake/v1", context, &attestation).unwrap());
    }

    #[test]
    fn verify_attestation_rejects_a_mismatched_attestation() {
        let registry = registry_with_fake_scheme();
        let context = b"error_code=EXAMPLE";
        let attestation = AttestationBytes::new(b"different".to_vec());

        assert!(!verify_attestation(&registry, "fake/v1", context, &attestation).unwrap());
    }

    #[test]
    fn verify_attestation_errors_on_unregistered_scheme_id() {
        let registry = registry_with_fake_scheme();
        let context = b"ctx";
        let attestation = AttestationBytes::default();

        let err = verify_attestation(&registry, "does-not-exist/v1", context, &attestation)
            .expect_err("unregistered scheme id must error");
        assert!(matches!(err, ProofError::UnknownScheme(id) if id == "does-not-exist/v1"));
    }

    #[test]
    fn verify_attestation_never_needs_a_concrete_backend_type() {
        // The whole point of this facade: this test file only ever
        // names `pq_proof_types`/`pq_proof_registry` types, never a
        // concrete backend's (e.g. arkworks-based) types.
        let registry = registry_with_fake_scheme();
        let ok = verify_attestation(
            &registry,
            "fake/v1",
            b"same",
            &AttestationBytes::new(b"same".to_vec()),
        );
        assert_eq!(ok, Ok(true));
    }
}
