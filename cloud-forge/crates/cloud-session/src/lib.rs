// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Temporary, expiring sessions -- the second piece of Phase 13's IAM
//! surface (`cloud-identity`'s Phase 1 doc comment: "credentials,
//! sessions, federation, policy documents ... is Phase 13").
//!
//! A [`Session`] is produced one of two ways -- assuming a role, or
//! federating in an external identity -- and is valid only while it is
//! both un-revoked *and* before its expiry. This is deliberately the
//! same two-independent-invalidity-path shape
//! [`cloud_visibility::MessageLease`] (Phase 7) already established
//! for at-least-once redelivery: there, a message became undeliverable
//! either by being currently in flight or by exceeding its receive
//! count; here, a session becomes invalid either by explicit
//! [`SessionStore::revoke`] or by simply outliving [`Session::expires_at`].
//! Neither path implies the other, and both must be checked.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_identity::Principal;
use cloud_types::{ResourceId, Timestamp};
use std::collections::BTreeMap;

/// How a session came to exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSource {
    /// The principal assumed a role, producing temporary credentials
    /// scoped to that role.
    AssumedRole { role: ResourceId },
    /// The principal was federated in from an external identity
    /// provider (SAML, OIDC, ...); `external_id` names that provider's
    /// own identifier for it.
    Federated { external_id: String },
}

/// A temporary session: a principal, how it got here, and the window
/// during which it's valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    token: ResourceId,
    principal: Principal,
    source: SessionSource,
    issued_at: Timestamp,
    expires_at: Timestamp,
    revoked: bool,
}

impl Session {
    pub fn token(&self) -> &ResourceId {
        &self.token
    }

    pub fn principal(&self) -> &Principal {
        &self.principal
    }

    pub fn source(&self) -> &SessionSource {
        &self.source
    }

    pub fn issued_at(&self) -> Timestamp {
        self.issued_at
    }

    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked
    }

    /// Whether this session may still be used at `now`: not revoked,
    /// and not yet expired.
    pub fn is_valid(&self, now: Timestamp) -> bool {
        !self.revoked && now < self.expires_at
    }
}

/// A registry of live sessions, keyed by their token.
#[derive(Debug, Clone, Default)]
pub struct SessionStore {
    sessions: BTreeMap<ResourceId, Session>,
}

impl SessionStore {
    pub fn new() -> Self {
        SessionStore {
            sessions: BTreeMap::new(),
        }
    }

    fn create(
        &mut self,
        token: ResourceId,
        principal: Principal,
        source: SessionSource,
        issued_at: Timestamp,
        duration_millis: u64,
    ) -> Result<&Session, CloudError> {
        if self.sessions.contains_key(&token) {
            return Err(CloudError::Conflict {
                what: "session",
                id: token.to_string(),
            });
        }
        let session = Session {
            token: token.clone(),
            principal,
            source,
            issued_at,
            expires_at: issued_at.saturating_add_millis(duration_millis),
            revoked: false,
        };
        self.sessions.insert(token.clone(), session);
        Ok(self
            .sessions
            .get(&token)
            .expect("just inserted under this exact id"))
    }

    /// Creates a session from a role assumption.
    pub fn assume_role(
        &mut self,
        token: ResourceId,
        principal: Principal,
        role: ResourceId,
        issued_at: Timestamp,
        duration_millis: u64,
    ) -> Result<&Session, CloudError> {
        self.create(
            token,
            principal,
            SessionSource::AssumedRole { role },
            issued_at,
            duration_millis,
        )
    }

    /// Creates a session from federating in an external identity.
    pub fn federate(
        &mut self,
        token: ResourceId,
        principal: Principal,
        external_id: impl Into<String>,
        issued_at: Timestamp,
        duration_millis: u64,
    ) -> Result<&Session, CloudError> {
        self.create(
            token,
            principal,
            SessionSource::Federated {
                external_id: external_id.into(),
            },
            issued_at,
            duration_millis,
        )
    }

    pub fn get(&self, token: &ResourceId) -> Option<&Session> {
        self.sessions.get(token)
    }

    pub fn is_valid(&self, token: &ResourceId, now: Timestamp) -> bool {
        self.sessions.get(token).is_some_and(|s| s.is_valid(now))
    }

    /// Explicitly revokes a session ahead of its natural expiry.
    pub fn revoke(&mut self, token: &ResourceId) -> Result<(), CloudError> {
        let session = self
            .sessions
            .get_mut(token)
            .ok_or_else(|| CloudError::NotFound {
                what: "session",
                id: token.to_string(),
            })?;
        session.revoked = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(name: &str) -> Principal {
        Principal::User(ResourceId::new(name).unwrap())
    }

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    #[test]
    fn a_freshly_assumed_role_session_is_valid_immediately() {
        let mut store = SessionStore::new();
        store
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                3_600_000,
            )
            .unwrap();
        assert!(store.is_valid(&id("tok-1"), Timestamp::from_millis(0)));
    }

    #[test]
    fn a_session_becomes_invalid_once_its_expiry_passes() {
        let mut store = SessionStore::new();
        store
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap();
        assert!(store.is_valid(&id("tok-1"), Timestamp::from_millis(999)));
        assert!(!store.is_valid(&id("tok-1"), Timestamp::from_millis(1000)));
    }

    #[test]
    fn revoking_invalidates_a_session_before_its_natural_expiry() {
        let mut store = SessionStore::new();
        store
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                3_600_000,
            )
            .unwrap();
        store.revoke(&id("tok-1")).unwrap();
        assert!(!store.is_valid(&id("tok-1"), Timestamp::from_millis(0)));
    }

    #[test]
    fn revoking_an_unknown_session_is_not_found() {
        let mut store = SessionStore::new();
        let err = store.revoke(&id("ghost")).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn a_duplicate_token_is_a_conflict() {
        let mut store = SessionStore::new();
        store
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap();
        let err = store
            .federate(
                id("tok-1"),
                principal("bob"),
                "external-idp-1",
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn federated_sessions_record_their_external_id() {
        let mut store = SessionStore::new();
        let session = store
            .federate(
                id("tok-1"),
                principal("alice"),
                "external-idp-1",
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap();
        assert_eq!(
            session.source(),
            &SessionSource::Federated {
                external_id: "external-idp-1".to_string()
            }
        );
    }

    #[test]
    fn assumed_role_sessions_record_the_role() {
        let mut store = SessionStore::new();
        let session = store
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap();
        assert_eq!(
            session.source(),
            &SessionSource::AssumedRole {
                role: id("role-admin")
            }
        );
    }

    #[test]
    fn is_valid_on_an_unknown_token_is_false_not_an_error() {
        let store = SessionStore::new();
        assert!(!store.is_valid(&id("ghost"), Timestamp::from_millis(0)));
    }

    #[test]
    fn expiry_and_revocation_are_independent_invalidity_paths() {
        let mut store = SessionStore::new();
        store
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap();
        // Not revoked, but expired.
        assert!(!store.get(&id("tok-1")).unwrap().is_revoked());
        assert!(!store.is_valid(&id("tok-1"), Timestamp::from_millis(2000)));

        store
            .assume_role(
                id("tok-2"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                3_600_000,
            )
            .unwrap();
        // Not expired, but revoked.
        store.revoke(&id("tok-2")).unwrap();
        assert!(!store.is_valid(&id("tok-2"), Timestamp::from_millis(0)));
    }
}
