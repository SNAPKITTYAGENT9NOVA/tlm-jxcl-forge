// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `StorageService`: the second real composed service in this
//! workspace, following exactly the shape `cloud-compute` (Phase 8)
//! established -- compose already-real primitives, introduce none.
//!
//! A volume is created with a chosen [`RedundancyScheme`] (Phase 5),
//! starts [`AttachmentState::Detached`] (Phase 5), and can be attached
//! to and detached from something repeatedly over its life. Deleting a
//! volume reconciles its `cloud-lifecycle` to `Deleted` exactly as
//! `cloud-compute::terminate` does, and -- the one real business rule
//! this service adds -- refuses to delete a volume that isn't
//! `Detached`, mirroring real block-storage services that won't let
//! you delete an in-use volume out from under whatever it's attached
//! to.
//!
//! Unlike `cloud-compute`, this service does **not** place volumes
//! onto a specific availability zone: there is no per-AZ byte-capacity
//! primitive yet (`CLOUD_ARCHITECTURE.md`'s Phase 5 section explains
//! why `cloud-storage-capacity` was deliberately not built alongside
//! the primitives it would need), so this phase tracks size only as
//! an account-level `cloud-quota` reservation (`"storage_bytes"`), the
//! same generic quota primitive `cloud-compute` uses for
//! `"instances"`. Per-AZ storage placement stays deferred until a
//! phase actually needs it.
#![forbid(unsafe_code)]

use cloud_account::AccountRegistry;
use cloud_attachment::AttachmentState;
use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_identity::Principal;
use cloud_lifecycle::Lifecycle;
use cloud_policy::Policy;
use cloud_quota::QuotaTracker;
use cloud_redundancy::RedundancyScheme;
use cloud_region::RegionRegistry;
use cloud_resource::Resource;
use cloud_resource_registry::ResourceRegistry;
use cloud_types::{AccountId, Arn, RegionId, ResourceId, ResourceType, Timestamp};
use std::collections::BTreeMap;

/// A volume's service-specific payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeSpec {
    pub size_bytes: u64,
    pub redundancy: RedundancyScheme,
}

/// A request to create one new volume.
pub struct CreateVolumeRequest {
    pub id: ResourceId,
    pub account: AccountId,
    pub region: RegionId,
    pub principal: Principal,
    pub size_bytes: u64,
    pub redundancy: RedundancyScheme,
    pub created_at: Timestamp,
}

/// The storage service: holds every registry `create_volume`/
/// `delete_volume` needs, and the store of volumes it has created.
pub struct StorageService {
    partition: String,
    service: String,
    accounts: AccountRegistry,
    regions: RegionRegistry,
    policy: Policy,
    quota: QuotaTracker,
    events: EventLog,
    names: ResourceRegistry,
    volumes: BTreeMap<ResourceId, Resource<VolumeSpec>>,
    attachment: BTreeMap<ResourceId, AttachmentState>,
}

impl StorageService {
    pub fn new(policy: Policy, partition: impl Into<String>, service: impl Into<String>) -> Self {
        StorageService {
            partition: partition.into(),
            service: service.into(),
            accounts: AccountRegistry::new(),
            regions: RegionRegistry::new(),
            policy,
            quota: QuotaTracker::new(),
            events: EventLog::new(),
            names: ResourceRegistry::new(),
            volumes: BTreeMap::new(),
            attachment: BTreeMap::new(),
        }
    }

    pub fn register_account(&mut self, account: cloud_account::Account) -> Result<(), CloudError> {
        self.accounts.register(account)
    }

    pub fn register_region(&mut self, region: RegionId, az_count: u32) -> Result<(), CloudError> {
        self.regions.register(region, az_count)
    }

    pub fn set_quota_limit(&mut self, resource_type: impl Into<String>, limit: u64) {
        self.quota.set_limit(resource_type, limit);
    }

    pub fn events(&self) -> &EventLog {
        &self.events
    }

    pub fn get(&self, id: &ResourceId) -> Option<&Resource<VolumeSpec>> {
        self.volumes.get(id)
    }

    pub fn list(&self) -> impl Iterator<Item = &Resource<VolumeSpec>> {
        self.volumes.values()
    }

    pub fn attachment_state(&self, id: &ResourceId) -> Option<AttachmentState> {
        self.attachment.get(id).copied()
    }

    pub fn resolve_arn(&self, arn: &Arn) -> Option<&Resource<VolumeSpec>> {
        self.names.resolve(arn).and_then(|id| self.volumes.get(id))
    }

    pub fn arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.names.arn_of(id)
    }

    /// Creates a new volume: validates the account and region exist,
    /// authorizes the request, reserves its size against the
    /// account's `"storage_bytes"` quota, constructs and names the
    /// resource, and starts it `Detached`. Any failure after the quota
    /// was reserved releases it -- a failed `create_volume` never
    /// leaves partial state behind.
    pub fn create_volume(
        &mut self,
        request: CreateVolumeRequest,
    ) -> Result<&Resource<VolumeSpec>, CloudError> {
        if !self.accounts.contains(&request.account) {
            return Err(CloudError::NotFound {
                what: "account",
                id: request.account.to_string(),
            });
        }
        if self.regions.get(&request.region).is_none() {
            return Err(CloudError::NotFound {
                what: "region",
                id: request.region.to_string(),
            });
        }
        if self.volumes.contains_key(&request.id) {
            return Err(CloudError::Conflict {
                what: "volume",
                id: request.id.to_string(),
            });
        }

        self.policy.authorize(
            &request.principal,
            "storage:create-volume",
            request.id.as_str(),
        )?;

        self.quota
            .try_reserve("storage_bytes", request.size_bytes)?;

        let resource_type = ResourceType::new("storage-volume").expect("valid literal");
        let resource = Resource::new(
            request.id.clone(),
            resource_type,
            Some(request.region.clone()),
            request.account.clone(),
            request.created_at,
            VolumeSpec {
                size_bytes: request.size_bytes,
                redundancy: request.redundancy,
            },
        );

        let arn = match Arn::new(
            self.partition.clone(),
            self.service.clone(),
            Some(request.region),
            Some(request.account),
            request.id.clone(),
        ) {
            Ok(arn) => arn,
            Err(e) => {
                self.quota.release("storage_bytes", request.size_bytes);
                return Err(e);
            }
        };
        if let Err(e) = self.names.register(&arn, request.id.clone()) {
            self.quota.release("storage_bytes", request.size_bytes);
            return Err(e);
        }

        self.events.append(
            request.id.clone(),
            "volume.created",
            request.created_at,
            format!(
                "account={} size_bytes={} shards={}",
                resource.account(),
                request.size_bytes,
                request.redundancy.total_shards()
            ),
        );

        self.volumes.insert(request.id.clone(), resource);
        self.attachment
            .insert(request.id.clone(), AttachmentState::Detached);
        Ok(self
            .volumes
            .get(&request.id)
            .expect("just inserted under this exact id"))
    }

    /// Attempts a single [`AttachmentState`] transition (e.g.
    /// `Detached` -> `Attaching`, `Attaching` -> `Attached`). Rejected
    /// transitions leave the stored state completely unchanged.
    pub fn transition_attachment(
        &mut self,
        id: &ResourceId,
        to: AttachmentState,
    ) -> Result<AttachmentState, CloudError> {
        let current = self
            .attachment
            .get(id)
            .copied()
            .ok_or_else(|| CloudError::NotFound {
                what: "volume",
                id: id.to_string(),
            })?;
        let next = current.transition_to(to)?;
        self.attachment.insert(id.clone(), next);
        Ok(next)
    }

    /// Deletes a volume: refuses unless it is currently `Detached`
    /// (mirroring real block-storage services, which won't delete a
    /// volume still attached to something), reconciles its lifecycle
    /// to `Deleted`, and releases the `"storage_bytes"` quota it held.
    pub fn delete_volume(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        let attachment = self
            .attachment
            .get(id)
            .copied()
            .ok_or_else(|| CloudError::NotFound {
                what: "volume",
                id: id.to_string(),
            })?;
        if attachment != AttachmentState::Detached {
            return Err(CloudError::Conflict {
                what: "volume",
                id: id.to_string(),
            });
        }

        let resource = self
            .volumes
            .get_mut(id)
            .expect("attachment and volume stores are always kept in sync");
        let plan = cloud_reconciler::reconcile(resource.lifecycle(), Lifecycle::Deleted)?;
        for step in plan {
            resource
                .transition_lifecycle(step, now)
                .expect("cloud-reconciler only ever returns already-valid single steps");
        }

        let size_bytes = resource.payload().size_bytes;
        self.quota.release("storage_bytes", size_bytes);
        self.events.append(
            id.clone(),
            "volume.deleted",
            now,
            format!("size_bytes={size_bytes}"),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_account::Account;
    use cloud_policy::{Effect, PrincipalMatcher, Statement};

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

    fn account_id() -> AccountId {
        AccountId::new("000000000001").unwrap()
    }

    fn region_id() -> RegionId {
        RegionId::new("us-west-1").unwrap()
    }

    fn setup() -> StorageService {
        let mut svc = StorageService::new(allow_all_policy(), "core", "storage");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.set_quota_limit("storage_bytes", 1_000_000);
        svc
    }

    fn create_request(id: &str) -> CreateVolumeRequest {
        CreateVolumeRequest {
            id: ResourceId::new(id).unwrap(),
            account: account_id(),
            region: region_id(),
            principal: Principal::User(ResourceId::new("alice").unwrap()),
            size_bytes: 1024,
            redundancy: RedundancyScheme::replicated(3).unwrap(),
            created_at: Timestamp::from_millis(1000),
        }
    }

    #[test]
    fn create_rejects_an_unregistered_account() {
        let mut svc = setup();
        let mut req = create_request("vol-1");
        req.account = AccountId::new("999999999999").unwrap();
        let err = svc.create_volume(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_rejects_an_unregistered_region() {
        let mut svc = setup();
        let mut req = create_request("vol-1");
        req.region = RegionId::new("eu-central-1").unwrap();
        let err = svc.create_volume(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_rejects_a_duplicate_id() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let err = svc.create_volume(create_request("vol-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn a_successful_create_reserves_quota_and_starts_detached() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Creating);
        assert_eq!(svc.attachment_state(&id), Some(AttachmentState::Detached));
        assert_eq!(svc.quota.usage("storage_bytes"), 1024);
    }

    #[test]
    fn create_registers_a_resolvable_arn() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        let arn = svc.arn_of(&id).unwrap();
        assert_eq!(svc.resolve_arn(&arn).unwrap().id(), &id);
    }

    #[test]
    fn exhausting_quota_fails_create_and_reserves_nothing() {
        let mut svc = setup();
        svc.set_quota_limit("storage_bytes", 100);
        let err = svc.create_volume(create_request("vol-1")).unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(svc.quota.usage("storage_bytes"), 0);
    }

    #[test]
    fn transition_attachment_moves_detached_to_attaching() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        let state = svc
            .transition_attachment(&id, AttachmentState::Attaching)
            .unwrap();
        assert_eq!(state, AttachmentState::Attaching);
        assert_eq!(svc.attachment_state(&id), Some(AttachmentState::Attaching));
    }

    #[test]
    fn transition_attachment_rejects_an_invalid_jump() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        let err = svc
            .transition_attachment(&id, AttachmentState::Attached)
            .unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert_eq!(svc.attachment_state(&id), Some(AttachmentState::Detached));
    }

    #[test]
    fn delete_volume_refuses_an_attached_volume() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        svc.transition_attachment(&id, AttachmentState::Attaching)
            .unwrap();
        svc.transition_attachment(&id, AttachmentState::Attached)
            .unwrap();

        let err = svc
            .delete_volume(&id, Timestamp::from_millis(2000))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        assert_eq!(
            svc.quota.usage("storage_bytes"),
            1024,
            "a refused delete must not release quota"
        );
    }

    #[test]
    fn delete_volume_releases_quota_and_reaches_deleted_when_detached() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();

        svc.delete_volume(&id, Timestamp::from_millis(2000))
            .unwrap();

        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Deleted);
        assert_eq!(svc.quota.usage("storage_bytes"), 0);
    }

    #[test]
    fn delete_volume_of_an_unknown_volume_is_not_found() {
        let mut svc = setup();
        let err = svc
            .delete_volume(
                &ResourceId::new("vol-ghost").unwrap(),
                Timestamp::from_millis(1000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn a_volume_can_be_reattached_after_being_detached_again() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        svc.transition_attachment(&id, AttachmentState::Attaching)
            .unwrap();
        svc.transition_attachment(&id, AttachmentState::Attached)
            .unwrap();
        svc.transition_attachment(&id, AttachmentState::Detaching)
            .unwrap();
        svc.transition_attachment(&id, AttachmentState::Detached)
            .unwrap();
        svc.delete_volume(&id, Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Deleted);
    }

    #[test]
    fn list_includes_deleted_volumes() {
        let mut svc = setup();
        svc.create_volume(create_request("vol-1")).unwrap();
        let id = ResourceId::new("vol-1").unwrap();
        svc.delete_volume(&id, Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(svc.list().count(), 1);
    }
}
