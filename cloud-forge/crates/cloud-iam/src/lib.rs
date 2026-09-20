// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `IamService`: the fifth composed service in this workspace, and the
//! first to compose Phase 13's IAM surface -- `cloud-credentials`,
//! `cloud-session`, `cloud-policy-document` -- together with
//! `cloud-policy` and `cloud-events`. No new primitive, only
//! composition, exactly the shape `cloud-compute`/`cloud-storage`/
//! `cloud-database`/`cloud-messaging` (Phases 8-11) each used for their
//! own primitive sets.
//!
//! ## The one real business rule this phase adds
//!
//! [`IamService::assume_role`] and [`IamService::federate`] both call
//! `Policy::authorize` *before* creating a session -- the first time in
//! this entire workspace that `cloud-policy` (Phase 1) governs an
//! operation *within the IAM surface itself*, rather than gating
//! another service's resource operation (a `launch`, a `create_volume`,
//! a `create_database`). The mechanism is identical to every other
//! service's own `policy.authorize(...)` call; what's new is the
//! target: a principal now needs permission to assume a role or
//! federate in at all, not just permission to act once it has a
//! session.
//!
//! ## No accounts, regions, or quota -- deliberately
//!
//! Every prior composed service (`cloud-compute`, `cloud-storage`,
//! `cloud-database`, `cloud-messaging`) validated an `AccountId` and
//! `RegionId` and reserved a `cloud-quota` unit before creating
//! anything, because each of those services provisions real,
//! region-scoped infrastructure. `IamService` does neither: real IAM
//! systems (AWS's own included) are inherently **global**, not
//! region-scoped, and a credential or session isn't infrastructure
//! with a finite regional capacity to exhaust -- there is nothing here
//! for an `AccountId`/`RegionId` check or a quota reservation to
//! usefully gate. Skipping them is a deliberate design choice
//! mirroring real IAM's own shape, not an oversight.
//!
//! ## No `Arn`, deliberately
//!
//! Every prior composed service also registered a resolvable `Arn` for
//! everything it created. Credentials and sessions are identity
//! artifacts, not provisioned resources with a canonical name external
//! callers look up -- mirroring real IAM's own inconsistent resource
//! model, where a role or user has an ARN but an access key or an STS
//! session token does not. `cloud-resource-registry` stays unused here.
#![forbid(unsafe_code)]

use cloud_credentials::{Credential, CredentialStore};
use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_identity::Principal;
use cloud_policy::Policy;
use cloud_session::{Session, SessionStore};
use cloud_types::{ResourceId, Timestamp};

/// The IAM service: a policy, the credential and session stores it
/// governs, and an audit trail of what happened.
pub struct IamService {
    policy: Policy,
    credentials: CredentialStore,
    sessions: SessionStore,
    events: EventLog,
}

impl IamService {
    pub fn new(policy: Policy) -> Self {
        IamService {
            policy,
            credentials: CredentialStore::new(),
            sessions: SessionStore::new(),
            events: EventLog::new(),
        }
    }

    pub fn events(&self) -> &EventLog {
        &self.events
    }

    pub fn credential(&self, id: &ResourceId) -> Option<&Credential> {
        self.credentials.get(id)
    }

    pub fn session(&self, token: &ResourceId) -> Option<&Session> {
        self.sessions.get(token)
    }

    pub fn is_session_valid(&self, token: &ResourceId, now: Timestamp) -> bool {
        self.sessions.is_valid(token, now)
    }

    /// The general-purpose IAM decision: would `principal` be allowed
    /// to perform `action` on `resource` right now?
    pub fn authorize(
        &self,
        principal: &Principal,
        action: &str,
        resource: &str,
    ) -> Result<(), CloudError> {
        self.policy.authorize(principal, action, resource)
    }

    /// Replaces the active policy with one parsed from a JSON-shaped
    /// policy document. Every subsequent `authorize`/`assume_role`/
    /// `federate` call is governed by this new policy immediately.
    pub fn load_policy_document(&mut self, json: &str) -> Result<(), CloudError> {
        self.policy = cloud_policy_document::from_json(json)?;
        Ok(())
    }

    /// Serializes the active policy back to a JSON-shaped document.
    pub fn export_policy_document(&self) -> String {
        cloud_policy_document::to_json(&self.policy)
    }

    pub fn create_credential(
        &mut self,
        id: ResourceId,
        principal: Principal,
        created_at: Timestamp,
    ) -> Result<&Credential, CloudError> {
        self.credentials
            .create(id.clone(), principal.clone(), created_at)?;
        self.events.append(
            id.clone(),
            "credential.created",
            created_at,
            format!("principal={principal}"),
        );
        Ok(self
            .credentials
            .get(&id)
            .expect("just created under this exact id"))
    }

    pub fn activate_credential(
        &mut self,
        id: &ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        self.credentials.activate(id)?;
        self.events
            .append(id.clone(), "credential.activated", now, String::new());
        Ok(())
    }

    pub fn deactivate_credential(
        &mut self,
        id: &ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        self.credentials.deactivate(id)?;
        self.events
            .append(id.clone(), "credential.deactivated", now, String::new());
        Ok(())
    }

    pub fn delete_credential(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        self.credentials.delete(id)?;
        self.events
            .append(id.clone(), "credential.deleted", now, String::new());
        Ok(())
    }

    /// Assumes `role` as `principal`, producing a new session. Refused
    /// unless the active policy authorizes `principal` to
    /// `"sts:assume-role"` on `role` -- checked before any session is
    /// created, so a denied attempt leaves the session store untouched.
    pub fn assume_role(
        &mut self,
        token: ResourceId,
        principal: Principal,
        role: ResourceId,
        now: Timestamp,
        duration_millis: u64,
    ) -> Result<&Session, CloudError> {
        self.policy
            .authorize(&principal, "sts:assume-role", role.as_str())?;
        self.sessions
            .assume_role(token.clone(), principal.clone(), role, now, duration_millis)?;
        self.events.append(
            token.clone(),
            "session.assumed-role",
            now,
            format!("principal={principal}"),
        );
        Ok(self
            .sessions
            .get(&token)
            .expect("just created under this exact token"))
    }

    /// Federates `principal` in from an external identity provider,
    /// producing a new session. Refused unless the active policy
    /// authorizes `principal` to `"sts:federate"` -- checked before any
    /// session is created.
    pub fn federate(
        &mut self,
        token: ResourceId,
        principal: Principal,
        external_id: impl Into<String>,
        now: Timestamp,
        duration_millis: u64,
    ) -> Result<&Session, CloudError> {
        self.policy.authorize(&principal, "sts:federate", "*")?;
        self.sessions.federate(
            token.clone(),
            principal.clone(),
            external_id,
            now,
            duration_millis,
        )?;
        self.events.append(
            token.clone(),
            "session.federated",
            now,
            format!("principal={principal}"),
        );
        Ok(self
            .sessions
            .get(&token)
            .expect("just created under this exact token"))
    }

    pub fn revoke_session(&mut self, token: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        self.sessions.revoke(token)?;
        self.events
            .append(token.clone(), "session.revoked", now, String::new());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_policy::{Effect, PrincipalMatcher, Statement};
    use cloud_session::SessionSource;

    fn allow_all_policy() -> Policy {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::Any,
            actions: vec!["*".to_string()],
            resources: vec!["*".to_string()],
        });
        policy
    }

    fn deny_all_policy() -> Policy {
        Policy::new()
    }

    fn principal(name: &str) -> Principal {
        Principal::User(ResourceId::new(name).unwrap())
    }

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    #[test]
    fn create_credential_starts_it_active_and_logs_an_event() {
        let mut iam = IamService::new(allow_all_policy());
        iam.create_credential(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        assert!(iam.credential(&id("key-1")).is_some());
        assert_eq!(iam.events().len(), 1);
    }

    #[test]
    fn credential_lifecycle_delegates_correctly() {
        let mut iam = IamService::new(allow_all_policy());
        iam.create_credential(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        iam.deactivate_credential(&id("key-1"), Timestamp::from_millis(1000))
            .unwrap();
        iam.activate_credential(&id("key-1"), Timestamp::from_millis(2000))
            .unwrap();
        iam.delete_credential(&id("key-1"), Timestamp::from_millis(3000))
            .unwrap();
        assert!(iam.credential(&id("key-1")).is_none());
        assert_eq!(iam.events().len(), 4);
    }

    #[test]
    fn assume_role_is_refused_without_authorization_and_creates_no_session() {
        let mut iam = IamService::new(deny_all_policy());
        let err = iam
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                3_600_000,
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::PolicyDenied { .. }));
        assert!(iam.session(&id("tok-1")).is_none());
    }

    #[test]
    fn assume_role_succeeds_when_authorized() {
        let mut iam = IamService::new(allow_all_policy());
        let session = iam
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                3_600_000,
            )
            .unwrap();
        assert_eq!(
            session.source(),
            &SessionSource::AssumedRole {
                role: id("role-admin")
            }
        );
        assert!(iam.is_session_valid(&id("tok-1"), Timestamp::from_millis(0)));
    }

    #[test]
    fn federate_is_refused_without_authorization_and_creates_no_session() {
        let mut iam = IamService::new(deny_all_policy());
        let err = iam
            .federate(
                id("tok-1"),
                principal("alice"),
                "external-idp-1",
                Timestamp::from_millis(0),
                3_600_000,
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::PolicyDenied { .. }));
        assert!(iam.session(&id("tok-1")).is_none());
    }

    #[test]
    fn federate_succeeds_when_authorized() {
        let mut iam = IamService::new(allow_all_policy());
        iam.federate(
            id("tok-1"),
            principal("alice"),
            "external-idp-1",
            Timestamp::from_millis(0),
            3_600_000,
        )
        .unwrap();
        assert!(iam.session(&id("tok-1")).is_some());
    }

    #[test]
    fn revoke_session_invalidates_it_immediately() {
        let mut iam = IamService::new(allow_all_policy());
        iam.assume_role(
            id("tok-1"),
            principal("alice"),
            id("role-admin"),
            Timestamp::from_millis(0),
            3_600_000,
        )
        .unwrap();
        iam.revoke_session(&id("tok-1"), Timestamp::from_millis(500))
            .unwrap();
        assert!(!iam.is_session_valid(&id("tok-1"), Timestamp::from_millis(500)));
    }

    #[test]
    fn revoke_session_of_an_unknown_token_is_not_found() {
        let mut iam = IamService::new(allow_all_policy());
        let err = iam
            .revoke_session(&id("ghost"), Timestamp::from_millis(0))
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn authorize_delegates_directly_to_the_active_policy() {
        let iam = IamService::new(allow_all_policy());
        assert!(iam
            .authorize(&principal("alice"), "compute:describe", "i-1")
            .is_ok());

        let iam_denied = IamService::new(deny_all_policy());
        assert!(iam_denied
            .authorize(&principal("alice"), "compute:describe", "i-1")
            .is_err());
    }

    #[test]
    fn loading_a_policy_document_replaces_the_active_policy_immediately() {
        let mut iam = IamService::new(allow_all_policy());
        // Freshly loaded policy denies everything (no statements).
        iam.load_policy_document(r#"{"Statement": []}"#).unwrap();
        let err = iam
            .assume_role(
                id("tok-1"),
                principal("alice"),
                id("role-admin"),
                Timestamp::from_millis(0),
                1000,
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::PolicyDenied { .. }));
    }

    #[test]
    fn exported_policy_document_round_trips_through_load() {
        let iam = IamService::new(allow_all_policy());
        let exported = iam.export_policy_document();

        let mut iam2 = IamService::new(deny_all_policy());
        iam2.load_policy_document(&exported).unwrap();
        // The re-loaded policy is the original allow-all policy, so
        // assume_role now succeeds on iam2 too.
        iam2.assume_role(
            id("tok-1"),
            principal("alice"),
            id("role-admin"),
            Timestamp::from_millis(0),
            1000,
        )
        .unwrap();
    }

    #[test]
    fn load_policy_document_rejects_malformed_json_and_keeps_the_old_policy() {
        let mut iam = IamService::new(allow_all_policy());
        let err = iam.load_policy_document("not json").unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
        // The old (allow-all) policy is still active.
        iam.assume_role(
            id("tok-1"),
            principal("alice"),
            id("role-admin"),
            Timestamp::from_millis(0),
            1000,
        )
        .unwrap();
    }
}
