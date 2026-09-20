// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A registry mapping scheme-id strings to boxed [`ProofScheme`]
//! implementations, supporting future proof schemes beyond
//! Groth16-Pedersen without any caller needing to know every concrete
//! backend type in advance.
//!
//! Owns: [`ProofRegistry::register`]/[`ProofRegistry::get`].
#![forbid(unsafe_code)]

use std::collections::HashMap;

pub use pq_proof_types::{AttestationBytes, ProofError, ProofScheme};

/// A registry of [`ProofScheme`] implementations keyed by their scheme
/// id. Holds heterogeneous schemes (different backends, different
/// concrete attestation types) behind one object-safe trait, so a
/// caller with a scheme id and some attestation bytes never needs to
/// import a specific backend crate (e.g. arkworks) to verify against
/// it -- see `pq-proof-verifier`'s facade.
#[derive(Default)]
pub struct ProofRegistry {
    schemes: HashMap<String, Box<dyn ProofScheme>>,
}

impl ProofRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        ProofRegistry {
            schemes: HashMap::new(),
        }
    }

    /// Register `scheme` under `scheme_id`. Registering a second scheme
    /// under an id already in use replaces the first, returning it.
    pub fn register(
        &mut self,
        scheme_id: &str,
        scheme: Box<dyn ProofScheme>,
    ) -> Option<Box<dyn ProofScheme>> {
        self.schemes.insert(scheme_id.to_string(), scheme)
    }

    /// Look up the scheme registered under `scheme_id`, if any.
    pub fn get(&self, scheme_id: &str) -> Option<&dyn ProofScheme> {
        self.schemes.get(scheme_id).map(|boxed| boxed.as_ref())
    }

    /// Whether any scheme is registered under `scheme_id`.
    pub fn contains(&self, scheme_id: &str) -> bool {
        self.schemes.contains_key(scheme_id)
    }

    /// How many schemes are currently registered.
    pub fn len(&self) -> usize {
        self.schemes.len()
    }

    /// Whether the registry has no registered schemes.
    pub fn is_empty(&self) -> bool {
        self.schemes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial fake `ProofScheme`, kept local to this crate's tests so
    /// this crate's tests stay fast and free of any real backend
    /// (arkworks or otherwise) or dependency-cycle risk on
    /// `pq-error-proof`.
    struct FakeScheme {
        id: &'static str,
        accept: bool,
    }

    impl ProofScheme for FakeScheme {
        fn scheme_id(&self) -> &'static str {
            self.id
        }

        fn verify(
            &self,
            _context: &[u8],
            _attestation: &AttestationBytes,
        ) -> Result<bool, ProofError> {
            Ok(self.accept)
        }
    }

    #[test]
    fn register_then_get_finds_the_same_scheme() {
        let mut registry = ProofRegistry::new();
        assert!(registry.is_empty());

        registry.register(
            "fake/v1",
            Box::new(FakeScheme {
                id: "fake/v1",
                accept: true,
            }),
        );

        let found = registry
            .get("fake/v1")
            .expect("scheme should be registered");
        assert_eq!(found.scheme_id(), "fake/v1");
        assert!(found.verify(b"ctx", &AttestationBytes::default()).unwrap());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("fake/v1"));
    }

    #[test]
    fn get_on_unknown_scheme_id_is_none() {
        let registry = ProofRegistry::new();
        assert!(registry.get("nope").is_none());
        assert!(!registry.contains("nope"));
    }

    #[test]
    fn registering_the_same_id_twice_replaces_and_returns_the_previous_scheme() {
        let mut registry = ProofRegistry::new();
        registry.register(
            "fake/v1",
            Box::new(FakeScheme {
                id: "fake/v1",
                accept: true,
            }),
        );

        let previous = registry.register(
            "fake/v1",
            Box::new(FakeScheme {
                id: "fake/v1",
                accept: false,
            }),
        );

        assert!(previous.is_some());
        assert_eq!(registry.len(), 1);
        let current = registry.get("fake/v1").unwrap();
        assert!(!current
            .verify(b"ctx", &AttestationBytes::default())
            .unwrap());
    }

    #[test]
    fn multiple_distinct_schemes_coexist() {
        let mut registry = ProofRegistry::new();
        registry.register(
            "accepts/v1",
            Box::new(FakeScheme {
                id: "accepts/v1",
                accept: true,
            }),
        );
        registry.register(
            "rejects/v1",
            Box::new(FakeScheme {
                id: "rejects/v1",
                accept: false,
            }),
        );

        assert!(registry
            .get("accepts/v1")
            .unwrap()
            .verify(b"ctx", &AttestationBytes::default())
            .unwrap());
        assert!(!registry
            .get("rejects/v1")
            .unwrap()
            .verify(b"ctx", &AttestationBytes::default())
            .unwrap());
        assert_eq!(registry.len(), 2);
    }
}
