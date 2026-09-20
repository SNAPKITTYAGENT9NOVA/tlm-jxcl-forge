// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Long-lived credential material bound to a [`Principal`]: the first
//! piece of Phase 13's IAM surface, which `cloud-identity`'s own Phase
//! 1 doc comment named as deferred ("credentials, sessions,
//! federation, policy documents ... is Phase 13 in the roadmap").
//!
//! This crate models the real IAM rule AWS itself enforces: a
//! principal may hold **at most two `Active` credentials at once**
//! (`Inactive` ones don't count against the cap, so a principal can
//! rotate by creating a new key, deactivating the old one, and
//! deleting it later without ever exceeding the limit). There is no
//! real key material here -- no secret bytes, no signing -- since
//! generating cryptographically-secure secrets needs a source of
//! randomness this zero-external-dependency workspace deliberately
//! doesn't pull in; this crate models the *lifecycle* of a credential
//! identity, exactly as `cloud-lifecycle` models a resource's lifecycle
//! without knowing what the resource does.
//!
//! Credential ids reuse `cloud_types::ResourceId` rather than a new
//! wrapper type -- the same opaque, validated string identifier this
//! workspace already uses for instance ids, volume ids, message ids,
//! and snapshot ids, and a credential id needs nothing more than that.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_identity::Principal;
use cloud_types::{ResourceId, Timestamp};
use std::collections::BTreeMap;

/// The real-world IAM limit: at most this many `Active` credentials
/// per principal at once.
pub const MAX_ACTIVE_CREDENTIALS_PER_PRINCIPAL: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStatus {
    Active,
    Inactive,
}

/// One credential's identity and lifecycle state -- no secret material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    id: ResourceId,
    principal: Principal,
    status: CredentialStatus,
    created_at: Timestamp,
}

impl Credential {
    pub fn id(&self) -> &ResourceId {
        &self.id
    }

    pub fn principal(&self) -> &Principal {
        &self.principal
    }

    pub fn status(&self) -> CredentialStatus {
        self.status
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
}

/// A registry of credentials, indexed both by id and by owning
/// principal.
#[derive(Debug, Clone, Default)]
pub struct CredentialStore {
    credentials: BTreeMap<ResourceId, Credential>,
    by_principal: BTreeMap<Principal, Vec<ResourceId>>,
}

impl CredentialStore {
    pub fn new() -> Self {
        CredentialStore {
            credentials: BTreeMap::new(),
            by_principal: BTreeMap::new(),
        }
    }

    pub fn get(&self, id: &ResourceId) -> Option<&Credential> {
        self.credentials.get(id)
    }

    pub fn for_principal(&self, principal: &Principal) -> impl Iterator<Item = &Credential> {
        self.by_principal
            .get(principal)
            .into_iter()
            .flatten()
            .filter_map(|id| self.credentials.get(id))
    }

    pub fn active_count(&self, principal: &Principal) -> usize {
        self.for_principal(principal)
            .filter(|c| c.status == CredentialStatus::Active)
            .count()
    }

    fn check_active_cap(&self, principal: &Principal) -> Result<(), CloudError> {
        let current = self.active_count(principal);
        if current >= MAX_ACTIVE_CREDENTIALS_PER_PRINCIPAL {
            return Err(CloudError::QuotaExceeded {
                resource: "active credentials".to_string(),
                limit: MAX_ACTIVE_CREDENTIALS_PER_PRINCIPAL as u64,
                requested: (current + 1) as u64,
            });
        }
        Ok(())
    }

    /// Creates a new, `Active` credential for `principal`. Rejects a
    /// duplicate `id`, and rejects a principal that already holds
    /// [`MAX_ACTIVE_CREDENTIALS_PER_PRINCIPAL`] active credentials --
    /// creating an inactive one is not a way around the cap, since a
    /// freshly created credential always starts `Active`.
    pub fn create(
        &mut self,
        id: ResourceId,
        principal: Principal,
        created_at: Timestamp,
    ) -> Result<&Credential, CloudError> {
        if self.credentials.contains_key(&id) {
            return Err(CloudError::Conflict {
                what: "credential",
                id: id.to_string(),
            });
        }
        self.check_active_cap(&principal)?;

        let credential = Credential {
            id: id.clone(),
            principal: principal.clone(),
            status: CredentialStatus::Active,
            created_at,
        };
        self.credentials.insert(id.clone(), credential);
        self.by_principal
            .entry(principal)
            .or_default()
            .push(id.clone());
        Ok(self
            .credentials
            .get(&id)
            .expect("just inserted under this exact id"))
    }

    /// Reactivates an `Inactive` credential. Rejected if doing so would
    /// push its principal over the active cap -- a principal cannot
    /// reactivate a third key while two others are already active.
    pub fn activate(&mut self, id: &ResourceId) -> Result<(), CloudError> {
        let principal = self
            .credentials
            .get(id)
            .ok_or_else(|| CloudError::NotFound {
                what: "credential",
                id: id.to_string(),
            })?
            .principal
            .clone();
        if self.credentials[id].status == CredentialStatus::Active {
            return Ok(());
        }
        self.check_active_cap(&principal)?;
        self.credentials.get_mut(id).unwrap().status = CredentialStatus::Active;
        Ok(())
    }

    /// Deactivates a credential. Always succeeds for a known id --
    /// deactivating can never violate the active cap.
    pub fn deactivate(&mut self, id: &ResourceId) -> Result<(), CloudError> {
        let credential = self
            .credentials
            .get_mut(id)
            .ok_or_else(|| CloudError::NotFound {
                what: "credential",
                id: id.to_string(),
            })?;
        credential.status = CredentialStatus::Inactive;
        Ok(())
    }

    /// Permanently removes a credential.
    pub fn delete(&mut self, id: &ResourceId) -> Result<(), CloudError> {
        let credential = self
            .credentials
            .remove(id)
            .ok_or_else(|| CloudError::NotFound {
                what: "credential",
                id: id.to_string(),
            })?;
        if let Some(list) = self.by_principal.get_mut(&credential.principal) {
            list.retain(|existing| existing != id);
        }
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
    fn creating_a_credential_starts_it_active() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        assert_eq!(
            store.get(&id("key-1")).unwrap().status(),
            CredentialStatus::Active
        );
    }

    #[test]
    fn creating_a_duplicate_id_is_a_conflict() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        let err = store
            .create(id("key-1"), principal("bob"), Timestamp::from_millis(0))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn a_third_active_credential_is_rejected() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store
            .create(id("key-2"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        let err = store
            .create(id("key-3"), principal("alice"), Timestamp::from_millis(0))
            .unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(store.active_count(&principal("alice")), 2);
    }

    #[test]
    fn different_principals_have_independent_caps() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store
            .create(id("key-2"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store
            .create(id("key-3"), principal("bob"), Timestamp::from_millis(0))
            .unwrap();
        assert_eq!(store.active_count(&principal("bob")), 1);
    }

    #[test]
    fn deactivating_frees_a_slot_under_the_cap() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store
            .create(id("key-2"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store.deactivate(&id("key-1")).unwrap();
        store
            .create(id("key-3"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        assert_eq!(store.active_count(&principal("alice")), 2);
    }

    #[test]
    fn reactivating_a_credential_can_be_rejected_by_the_cap() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store
            .create(id("key-2"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store.deactivate(&id("key-1")).unwrap();
        store
            .create(id("key-3"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        // alice now has key-2 and key-3 active, key-1 inactive.
        let err = store.activate(&id("key-1")).unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
    }

    #[test]
    fn reactivating_an_already_active_credential_is_a_no_op() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store.activate(&id("key-1")).unwrap();
        assert_eq!(store.active_count(&principal("alice")), 1);
    }

    #[test]
    fn deleting_a_credential_removes_it_from_its_principal() {
        let mut store = CredentialStore::new();
        store
            .create(id("key-1"), principal("alice"), Timestamp::from_millis(0))
            .unwrap();
        store.delete(&id("key-1")).unwrap();
        assert!(store.get(&id("key-1")).is_none());
        assert_eq!(store.for_principal(&principal("alice")).count(), 0);
    }

    #[test]
    fn operations_on_an_unknown_credential_are_not_found() {
        let mut store = CredentialStore::new();
        assert!(matches!(
            store.activate(&id("ghost")).unwrap_err(),
            CloudError::NotFound { .. }
        ));
        assert!(matches!(
            store.deactivate(&id("ghost")).unwrap_err(),
            CloudError::NotFound { .. }
        ));
        assert!(matches!(
            store.delete(&id("ghost")).unwrap_err(),
            CloudError::NotFound { .. }
        ));
    }
}
