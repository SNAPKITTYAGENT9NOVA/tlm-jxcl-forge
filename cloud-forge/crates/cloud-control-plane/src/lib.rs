// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `ControlPlane<T>`: the Phase 2/3 orchestrating facade.
//!
//! Where `cloud-core` (Phase 1) is a pure re-export with no state of
//! its own, `ControlPlane<T>` actually holds the account and region
//! registries, the policy, the quota tracker, the event log, a
//! resource-name (`Arn`) registry, and a resource store, and is where
//! `create`/`get`/`list`/`delete` become real operations rather than
//! the caller's own responsibility to wire together every time.
#![forbid(unsafe_code)]

use cloud_account::{Account, AccountRegistry};
use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_lifecycle::Lifecycle;
use cloud_policy::Policy;
use cloud_provisioner::{provision, release_reservation, ProvisionContext, ProvisionRequest};
use cloud_quota::QuotaTracker;
use cloud_region::RegionRegistry;
use cloud_resource::Resource;
use cloud_resource_registry::ResourceRegistry;
use cloud_types::{Arn, AzId, RegionId, ResourceId, Timestamp};
use std::collections::BTreeMap;

/// The control plane for one deployment: registries, policy, quota,
/// events, and the resources it has provisioned.
pub struct ControlPlane<T> {
    /// This control plane's own ARN partition (e.g. `"core"`).
    partition: String,
    /// This control plane's own ARN service name (e.g. `"compute"`).
    service: String,
    accounts: AccountRegistry,
    regions: RegionRegistry,
    policy: Policy,
    quota: QuotaTracker,
    events: EventLog,
    az_usage: BTreeMap<AzId, u64>,
    resource_names: ResourceRegistry,
    resources: BTreeMap<ResourceId, Resource<T>>,
}

impl<T> ControlPlane<T> {
    /// `partition` and `service` become every provisioned resource's
    /// `Arn` prefix (`jxcl:cloud:<partition>:<service>:...`) -- they
    /// are validated lazily, the first time `create()` actually
    /// builds an `Arn`, rather than here.
    pub fn new(policy: Policy, partition: impl Into<String>, service: impl Into<String>) -> Self {
        ControlPlane {
            partition: partition.into(),
            service: service.into(),
            accounts: AccountRegistry::new(),
            regions: RegionRegistry::new(),
            policy,
            quota: QuotaTracker::new(),
            events: EventLog::new(),
            az_usage: BTreeMap::new(),
            resource_names: ResourceRegistry::new(),
            resources: BTreeMap::new(),
        }
    }

    pub fn register_account(&mut self, account: Account) -> Result<(), CloudError> {
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

    /// Creates a resource: verifies the request's account is
    /// registered and its id isn't already in use, then runs
    /// `cloud-provisioner`'s pipeline (quota is tracked per
    /// `resource_type`), computes and registers the resource's
    /// canonical `Arn`, and stores the result. If ARN construction or
    /// registration fails after `provision` already succeeded, the
    /// quota reservation and placement it made are rolled back before
    /// the error returns -- the same discipline as every other
    /// `create()` failure mode.
    pub fn create(&mut self, request: ProvisionRequest<T>) -> Result<&Resource<T>, CloudError> {
        if !self.accounts.contains(&request.account) {
            return Err(CloudError::NotFound {
                what: "account",
                id: request.account.to_string(),
            });
        }
        if self.resources.contains_key(&request.id) {
            return Err(CloudError::Conflict {
                what: "resource",
                id: request.id.to_string(),
            });
        }

        let quota_resource = request.resource_type.as_str().to_string();
        let outcome = provision(
            request,
            &mut ProvisionContext {
                policy: &self.policy,
                quota: &mut self.quota,
                quota_resource: &quota_resource,
                region_registry: &self.regions,
                az_usage: &mut self.az_usage,
                events: &mut self.events,
            },
        )?;

        let arn = match Arn::new(
            self.partition.clone(),
            self.service.clone(),
            outcome.resource.region().cloned(),
            Some(outcome.resource.account().clone()),
            outcome.resource.id().clone(),
        ) {
            Ok(arn) => arn,
            Err(e) => {
                release_reservation(
                    &mut self.quota,
                    &quota_resource,
                    &mut self.az_usage,
                    &outcome.az,
                );
                return Err(e);
            }
        };
        if let Err(e) = self
            .resource_names
            .register(&arn, outcome.resource.id().clone())
        {
            release_reservation(
                &mut self.quota,
                &quota_resource,
                &mut self.az_usage,
                &outcome.az,
            );
            return Err(e);
        }

        let id = outcome.resource.id().clone();
        self.resources.insert(id.clone(), outcome.resource);
        Ok(self
            .resources
            .get(&id)
            .expect("just inserted under this exact id"))
    }

    /// The resource `arn` names, if it has been provisioned by this
    /// control plane.
    pub fn resolve_arn(&self, arn: &Arn) -> Option<&Resource<T>> {
        self.resource_names
            .resolve(arn)
            .and_then(|id| self.resources.get(id))
    }

    /// `id`'s canonical `Arn`, if it was provisioned by this control
    /// plane (and so has one registered).
    pub fn arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.resource_names.arn_of(id)
    }

    pub fn get(&self, id: &ResourceId) -> Option<&Resource<T>> {
        self.resources.get(id)
    }

    /// Every resource this control plane has provisioned, in
    /// deterministic (id-sorted) order -- including deleted ones,
    /// which remain in the store with `Lifecycle::Deleted` rather
    /// than being removed, so a deleted resource's id can never be
    /// silently reused for something else (a duplicate `create` with
    /// the same id is always a conflict, regardless of the original
    /// resource's current lifecycle).
    pub fn list(&self) -> impl Iterator<Item = &Resource<T>> {
        self.resources.values()
    }

    /// Deletes a resource: reconciles its current lifecycle to
    /// `Deleted` (via `cloud-reconciler`, so this works regardless of
    /// which lifecycle state the resource is currently in) and
    /// releases the quota unit its creation reserved. Fails if the
    /// resource doesn't exist, or already is `Deleted`.
    pub fn delete(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        let resource = self
            .resources
            .get_mut(id)
            .ok_or_else(|| CloudError::NotFound {
                what: "resource",
                id: id.to_string(),
            })?;

        if resource.lifecycle() == Lifecycle::Deleted {
            return Err(CloudError::Conflict {
                what: "resource",
                id: id.to_string(),
            });
        }

        let plan = cloud_reconciler::reconcile(resource.lifecycle(), Lifecycle::Deleted)?;
        for step in plan {
            resource.transition_lifecycle(step, now)?;
        }

        self.quota.release(resource.resource_type().as_str(), 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_identity::Principal;
    use cloud_policy::{Effect, PrincipalMatcher, Statement};
    use cloud_types::{AccountId, ResourceType, Timestamp};

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

    fn setup() -> ControlPlane<u32> {
        let mut cp = ControlPlane::new(allow_all_policy(), "core", "compute");
        cp.register_account(Account::new(account_id(), "demo", Timestamp::EPOCH).unwrap())
            .unwrap();
        cp.register_region(RegionId::new("us-west-1").unwrap(), 2)
            .unwrap();
        cp.set_quota_limit("compute-instance", 5);
        cp
    }

    fn request(id: &str) -> ProvisionRequest<u32> {
        ProvisionRequest {
            id: ResourceId::new(id).unwrap(),
            resource_type: ResourceType::new("compute-instance").unwrap(),
            account: account_id(),
            principal: Principal::User(ResourceId::new("alice").unwrap()),
            action: "compute:create".to_string(),
            region: RegionId::new("us-west-1").unwrap(),
            created_at: Timestamp::from_millis(1_000),
            payload: 7,
            tags: Vec::new(),
        }
    }

    #[test]
    fn create_rejects_an_unregistered_account() {
        let mut cp: ControlPlane<u32> = ControlPlane::new(allow_all_policy(), "core", "compute");
        cp.register_region(RegionId::new("us-west-1").unwrap(), 1)
            .unwrap();
        let err = cp.create(request("i-1")).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_then_get_and_list_reflect_the_new_resource() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        assert!(cp.get(&ResourceId::new("i-1").unwrap()).is_some());
        assert_eq!(cp.list().count(), 1);
        assert_eq!(cp.events().len(), 1);
    }

    #[test]
    fn create_rejects_a_duplicate_id() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        let err = cp.create(request("i-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn delete_reconciles_from_any_reachable_lifecycle_and_releases_quota() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        assert_eq!(cp.quota.usage("compute-instance"), 1);
        // The resource is freshly created, so it's still `Creating` --
        // deleting it must reconcile through Active/Failed first, not
        // fail outright.
        assert_eq!(
            cp.get(&ResourceId::new("i-1").unwrap())
                .unwrap()
                .lifecycle(),
            Lifecycle::Creating
        );

        cp.delete(
            &ResourceId::new("i-1").unwrap(),
            Timestamp::from_millis(2_000),
        )
        .unwrap();

        assert_eq!(
            cp.get(&ResourceId::new("i-1").unwrap())
                .unwrap()
                .lifecycle(),
            Lifecycle::Deleted
        );
        assert_eq!(cp.quota.usage("compute-instance"), 0);
    }

    #[test]
    fn delete_is_idempotent_proof_against_double_delete() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        cp.delete(
            &ResourceId::new("i-1").unwrap(),
            Timestamp::from_millis(2_000),
        )
        .unwrap();
        let err = cp
            .delete(
                &ResourceId::new("i-1").unwrap(),
                Timestamp::from_millis(3_000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn delete_of_a_nonexistent_resource_is_not_found() {
        let mut cp = setup();
        let err = cp
            .delete(
                &ResourceId::new("does-not-exist").unwrap(),
                Timestamp::EPOCH,
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn a_deleted_resources_id_can_never_be_reused() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        cp.delete(
            &ResourceId::new("i-1").unwrap(),
            Timestamp::from_millis(2_000),
        )
        .unwrap();
        // The resource is gone (Deleted), but the id is still on
        // record in the store -- CLOUD-I003: deleted resources cannot
        // silently reappear, which this enforces by making the id
        // permanently unavailable for reuse.
        let err = cp.create(request("i-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn list_includes_deleted_resources() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        cp.delete(
            &ResourceId::new("i-1").unwrap(),
            Timestamp::from_millis(2_000),
        )
        .unwrap();
        assert_eq!(cp.list().count(), 1);
    }

    #[test]
    fn create_registers_a_resolvable_arn() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();

        let expected = Arn::new(
            "core",
            "compute",
            Some(RegionId::new("us-west-1").unwrap()),
            Some(account_id()),
            ResourceId::new("i-1").unwrap(),
        )
        .unwrap();

        assert_eq!(
            cp.arn_of(&ResourceId::new("i-1").unwrap()),
            Some(expected.clone())
        );
        assert_eq!(
            cp.resolve_arn(&expected).unwrap().id(),
            &ResourceId::new("i-1").unwrap()
        );
    }

    #[test]
    fn resolve_arn_of_an_unprovisioned_resource_is_none() {
        let cp = setup();
        let arn = Arn::new(
            "core",
            "compute",
            Some(RegionId::new("us-west-1").unwrap()),
            Some(account_id()),
            ResourceId::new("does-not-exist").unwrap(),
        )
        .unwrap();
        assert!(cp.resolve_arn(&arn).is_none());
    }

    #[test]
    fn an_arn_remains_resolvable_after_the_resource_it_names_is_deleted() {
        let mut cp = setup();
        cp.create(request("i-1")).unwrap();
        let arn = cp.arn_of(&ResourceId::new("i-1").unwrap()).unwrap();

        cp.delete(
            &ResourceId::new("i-1").unwrap(),
            Timestamp::from_millis(2_000),
        )
        .unwrap();

        // The name still resolves -- to the now-Deleted tombstone,
        // consistent with CLOUD-I003 (nothing about a resource,
        // including its name, silently disappears).
        assert_eq!(
            cp.resolve_arn(&arn).unwrap().lifecycle(),
            Lifecycle::Deleted
        );
    }
}
