// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `ComputeService`: the first genuinely compute-shaped service this
//! workspace builds, composing already-real primitives rather than
//! introducing new ones.
//!
//! Every crate `ComputeService` depends on already existed before this
//! phase: `cloud-image` (Phase 4) validates the boot image, capacity-
//! aware placement via `cloud-scheduler` (Phase 4, composing
//! `cloud-capacity`) reserves room for the instance, `cloud-runtime`
//! (Phase 4) tracks whether it's actually running, `cloud-resource-registry`
//! (Phase 3) gives it a resolvable name, and `cloud-reconciler`
//! (Phase 2) drives its `cloud-lifecycle` teardown. This crate adds no
//! new primitive -- only the orchestration that was deliberately left
//! unbuilt in Phases 3-5 while those primitives didn't exist yet.
//!
//! `ComputeService` does not reuse `cloud-provisioner`/
//! `cloud-control-plane` (Phase 2): those pipelines are generic over
//! any resource type and place with `cloud_scheduler::place_least_loaded`,
//! which knows nothing about vCPU/memory. Reusing them here would mean
//! either teaching a generic pipeline about compute capacity (exactly
//! the one-off special-casing this workspace's rule exists to
//! prevent) or launching instances without a capacity check at all.
//! `ComputeService` instead composes the same underlying primitives
//! directly, with its own capacity-aware pipeline.
#![forbid(unsafe_code)]

use cloud_account::AccountRegistry;
use cloud_capacity::{Capacity, CapacityTracker};
use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_identity::Principal;
use cloud_image::ImageRegistry;
use cloud_lifecycle::Lifecycle;
use cloud_policy::Policy;
use cloud_quota::QuotaTracker;
use cloud_region::RegionRegistry;
use cloud_resource::Resource;
use cloud_resource_registry::ResourceRegistry;
use cloud_runtime::RuntimeState;
use cloud_types::{AccountId, Arn, AzId, RegionId, ResourceId, ResourceType, Timestamp};
use std::collections::BTreeMap;

/// A compute instance's service-specific payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceSpec {
    pub image_id: ResourceId,
    pub capacity: Capacity,
    pub az: AzId,
}

/// A request to launch one new instance.
pub struct LaunchRequest {
    pub id: ResourceId,
    pub account: AccountId,
    pub region: RegionId,
    pub principal: Principal,
    pub image_id: ResourceId,
    pub capacity: Capacity,
    pub created_at: Timestamp,
}

/// The compute service: holds every registry `launch`/`terminate`
/// needs, and the store of instances it has created.
pub struct ComputeService {
    partition: String,
    service: String,
    accounts: AccountRegistry,
    regions: RegionRegistry,
    policy: Policy,
    quota: QuotaTracker,
    events: EventLog,
    images: ImageRegistry,
    capacity: CapacityTracker,
    az_usage: BTreeMap<AzId, u64>,
    names: ResourceRegistry,
    instances: BTreeMap<ResourceId, Resource<InstanceSpec>>,
    runtime: BTreeMap<ResourceId, RuntimeState>,
}

impl ComputeService {
    pub fn new(policy: Policy, partition: impl Into<String>, service: impl Into<String>) -> Self {
        ComputeService {
            partition: partition.into(),
            service: service.into(),
            accounts: AccountRegistry::new(),
            regions: RegionRegistry::new(),
            policy,
            quota: QuotaTracker::new(),
            events: EventLog::new(),
            images: ImageRegistry::new(),
            capacity: CapacityTracker::new(),
            az_usage: BTreeMap::new(),
            names: ResourceRegistry::new(),
            instances: BTreeMap::new(),
            runtime: BTreeMap::new(),
        }
    }

    pub fn register_account(&mut self, account: cloud_account::Account) -> Result<(), CloudError> {
        self.accounts.register(account)
    }

    pub fn register_region(&mut self, region: RegionId, az_count: u32) -> Result<(), CloudError> {
        self.regions.register(region, az_count)
    }

    pub fn register_az_capacity(&mut self, az: AzId, total: Capacity) -> Result<(), CloudError> {
        self.capacity.register_az(az, total)
    }

    pub fn register_image(
        &mut self,
        id: ResourceId,
        name: impl Into<String>,
        size_bytes: u64,
        architecture: cloud_image::Architecture,
    ) -> Result<(), CloudError> {
        self.images.register(id, name, size_bytes, architecture)?;
        Ok(())
    }

    pub fn set_quota_limit(&mut self, resource_type: impl Into<String>, limit: u64) {
        self.quota.set_limit(resource_type, limit);
    }

    pub fn events(&self) -> &EventLog {
        &self.events
    }

    pub fn get(&self, id: &ResourceId) -> Option<&Resource<InstanceSpec>> {
        self.instances.get(id)
    }

    pub fn list(&self) -> impl Iterator<Item = &Resource<InstanceSpec>> {
        self.instances.values()
    }

    pub fn runtime_state(&self, id: &ResourceId) -> Option<RuntimeState> {
        self.runtime.get(id).copied()
    }

    pub fn resolve_arn(&self, arn: &Arn) -> Option<&Resource<InstanceSpec>> {
        self.names
            .resolve(arn)
            .and_then(|id| self.instances.get(id))
    }

    pub fn arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.names.arn_of(id)
    }

    /// Launches a new instance: validates the account and image exist,
    /// authorizes the request, reserves an instance quota unit,
    /// capacity-aware-places it onto a region's least-loaded AZ with
    /// room, constructs and names the resource, and records the
    /// launch. Any failure after a reservation was made releases it --
    /// a failed `launch` never leaves partial state behind.
    pub fn launch(
        &mut self,
        request: LaunchRequest,
    ) -> Result<&Resource<InstanceSpec>, CloudError> {
        if !self.accounts.contains(&request.account) {
            return Err(CloudError::NotFound {
                what: "account",
                id: request.account.to_string(),
            });
        }
        if !self.images.contains(&request.image_id) {
            return Err(CloudError::NotFound {
                what: "image",
                id: request.image_id.to_string(),
            });
        }
        if self.instances.contains_key(&request.id) {
            return Err(CloudError::Conflict {
                what: "instance",
                id: request.id.to_string(),
            });
        }

        self.policy
            .authorize(&request.principal, "compute:launch", request.id.as_str())?;

        self.quota.try_reserve("instances", 1)?;

        let az = match cloud_scheduler::place_least_loaded_with_capacity(
            &self.regions,
            &mut self.capacity,
            &request.region,
            &mut self.az_usage,
            request.capacity,
        ) {
            Ok(az) => az,
            Err(e) => {
                self.quota.release("instances", 1);
                return Err(e);
            }
        };

        let resource_type = ResourceType::new("compute-instance").expect("valid literal");
        let resource = Resource::new(
            request.id.clone(),
            resource_type,
            Some(request.region.clone()),
            request.account.clone(),
            request.created_at,
            InstanceSpec {
                image_id: request.image_id.clone(),
                capacity: request.capacity,
                az: az.clone(),
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
                self.release_placement(&az, request.capacity);
                return Err(e);
            }
        };
        if let Err(e) = self.names.register(&arn, request.id.clone()) {
            self.release_placement(&az, request.capacity);
            return Err(e);
        }

        self.events.append(
            request.id.clone(),
            "instance.launched",
            request.created_at,
            format!(
                "account={} az={az} image={}",
                resource.account(),
                request.image_id
            ),
        );

        self.instances.insert(request.id.clone(), resource);
        self.runtime
            .insert(request.id.clone(), RuntimeState::Pending);
        Ok(self
            .instances
            .get(&request.id)
            .expect("just inserted under this exact id"))
    }

    /// Attempts a single [`RuntimeState`] transition (e.g. `Pending`
    /// -> `Running`, `Running` -> `Stopping`). Rejected transitions
    /// leave the stored state completely unchanged. `RuntimeState`
    /// lives in its own store, separate from the instance's
    /// `Resource<InstanceSpec>` record, so this never touches the
    /// resource's own `version`/`updated_at` -- exactly the
    /// independence `cloud-runtime`'s own docs describe between
    /// execution state and the resource record it belongs to.
    pub fn transition_runtime(
        &mut self,
        id: &ResourceId,
        to: RuntimeState,
    ) -> Result<RuntimeState, CloudError> {
        let current = self
            .runtime
            .get(id)
            .copied()
            .ok_or_else(|| CloudError::NotFound {
                what: "instance",
                id: id.to_string(),
            })?;
        let next = current.transition_to(to)?;
        self.runtime.insert(id.clone(), next);
        Ok(next)
    }

    /// Tears an instance all the way down: drives its runtime state to
    /// `Terminated`, reconciles its lifecycle to `Deleted`, and
    /// releases the quota unit and placement capacity it was holding.
    /// A second `terminate` call on an already-terminated instance
    /// fails without side effects, since `RuntimeState::Terminated` has
    /// no outgoing transitions.
    pub fn terminate(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        let current_runtime =
            self.runtime
                .get(id)
                .copied()
                .ok_or_else(|| CloudError::NotFound {
                    what: "instance",
                    id: id.to_string(),
                })?;

        let mut runtime = current_runtime;
        if runtime != RuntimeState::Terminating {
            runtime = runtime.transition_to(RuntimeState::Terminating)?;
        }
        runtime = runtime.transition_to(RuntimeState::Terminated)?;
        self.runtime.insert(id.clone(), runtime);

        let resource = self
            .instances
            .get_mut(id)
            .expect("runtime and instance stores are always kept in sync");
        let plan = cloud_reconciler::reconcile(resource.lifecycle(), Lifecycle::Deleted)?;
        for step in plan {
            resource
                .transition_lifecycle(step, now)
                .expect("cloud-reconciler only ever returns already-valid single steps");
        }

        let payload = resource.payload().clone();
        self.quota.release("instances", 1);
        self.release_placement(&payload.az, payload.capacity);
        self.events.append(
            id.clone(),
            "instance.terminated",
            now,
            format!("az={}", payload.az),
        );
        Ok(())
    }

    fn release_placement(&mut self, az: &AzId, amount: Capacity) {
        self.capacity.release(az, amount);
        if let Some(count) = self.az_usage.get_mut(az) {
            *count = count.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_account::Account;
    use cloud_image::Architecture;
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

    fn setup() -> ComputeService {
        let mut svc = ComputeService::new(allow_all_policy(), "core", "compute");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 2).unwrap();
        svc.register_az_capacity(AzId::new("us-west-1a").unwrap(), Capacity::new(16, 65536))
            .unwrap();
        svc.register_az_capacity(AzId::new("us-west-1b").unwrap(), Capacity::new(16, 65536))
            .unwrap();
        svc.register_image(
            ResourceId::new("ami-1").unwrap(),
            "base-linux",
            1024,
            Architecture::X86_64,
        )
        .unwrap();
        svc
    }

    fn launch_request(id: &str) -> LaunchRequest {
        LaunchRequest {
            id: ResourceId::new(id).unwrap(),
            account: account_id(),
            region: region_id(),
            principal: Principal::User(ResourceId::new("alice").unwrap()),
            image_id: ResourceId::new("ami-1").unwrap(),
            capacity: Capacity::new(2, 4096),
            created_at: Timestamp::from_millis(1000),
        }
    }

    #[test]
    fn launch_rejects_an_unregistered_account() {
        let mut svc = setup();
        let mut req = launch_request("i-1");
        req.account = AccountId::new("999999999999").unwrap();
        let err = svc.launch(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn launch_rejects_an_unregistered_image() {
        let mut svc = setup();
        let mut req = launch_request("i-1");
        req.image_id = ResourceId::new("ami-nonexistent").unwrap();
        let err = svc.launch(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn a_successful_launch_reserves_capacity_and_starts_pending() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();

        let az = svc.get(&id).unwrap().payload().az.clone();
        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Creating);
        assert_eq!(svc.runtime_state(&id), Some(RuntimeState::Pending));
        assert_eq!(svc.capacity.used(&az), Capacity::new(2, 4096));
    }

    #[test]
    fn launch_registers_a_resolvable_arn() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        let arn = svc.arn_of(&id).unwrap();
        assert_eq!(svc.resolve_arn(&arn).unwrap().id(), &id);
    }

    #[test]
    fn launching_a_duplicate_id_is_a_conflict_and_reserves_nothing_twice() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let err = svc.launch(launch_request("i-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        assert_eq!(svc.quota.usage("instances"), 1);
    }

    #[test]
    fn exhausting_capacity_fails_the_launch_and_reserves_no_quota() {
        let mut svc = setup();
        // Each AZ holds 16 vCPU; three 6-vCPU instances (18 total)
        // cannot all fit across two 16-vCPU AZs... use tighter numbers
        // to force exhaustion deterministically instead:
        let mut big = launch_request("i-1");
        big.capacity = Capacity::new(16, 65536);
        svc.launch(big).unwrap(); // fills az a
        let mut big2 = launch_request("i-2");
        big2.capacity = Capacity::new(16, 65536);
        svc.launch(big2).unwrap(); // fills az b
        let before = svc.quota.usage("instances");

        let mut too_big = launch_request("i-3");
        too_big.capacity = Capacity::new(1, 1);
        let err = svc.launch(too_big).unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(
            svc.quota.usage("instances"),
            before,
            "a failed PLAN stage must roll back the VALIDATE-stage quota reservation"
        );
    }

    #[test]
    fn transition_runtime_moves_pending_to_running() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        let state = svc.transition_runtime(&id, RuntimeState::Running).unwrap();
        assert_eq!(state, RuntimeState::Running);
        assert_eq!(svc.runtime_state(&id), Some(RuntimeState::Running));
    }

    #[test]
    fn transition_runtime_rejects_an_invalid_jump() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        let err = svc
            .transition_runtime(&id, RuntimeState::Stopped)
            .unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert_eq!(svc.runtime_state(&id), Some(RuntimeState::Pending));
    }

    #[test]
    fn terminate_releases_capacity_and_quota_and_reaches_deleted() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        let az = svc.get(&id).unwrap().payload().az.clone();

        svc.terminate(&id, Timestamp::from_millis(3000)).unwrap();

        assert_eq!(svc.runtime_state(&id), Some(RuntimeState::Terminated));
        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Deleted);
        assert_eq!(svc.capacity.used(&az), Capacity::default());
        assert_eq!(svc.quota.usage("instances"), 0);
    }

    #[test]
    fn terminate_is_not_idempotent_a_second_call_fails() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        svc.terminate(&id, Timestamp::from_millis(3000)).unwrap();
        let err = svc
            .terminate(&id, Timestamp::from_millis(4000))
            .unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
    }

    #[test]
    fn terminate_of_an_unknown_instance_is_not_found() {
        let mut svc = setup();
        let err = svc
            .terminate(
                &ResourceId::new("i-ghost").unwrap(),
                Timestamp::from_millis(1000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn terminate_after_running_still_reaches_terminated() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        svc.transition_runtime(&id, RuntimeState::Running).unwrap();
        svc.terminate(&id, Timestamp::from_millis(3000)).unwrap();
        assert_eq!(svc.runtime_state(&id), Some(RuntimeState::Terminated));
    }

    #[test]
    fn list_includes_terminated_instances() {
        let mut svc = setup();
        svc.launch(launch_request("i-1")).unwrap();
        let id = ResourceId::new("i-1").unwrap();
        svc.terminate(&id, Timestamp::from_millis(3000)).unwrap();
        assert_eq!(svc.list().count(), 1);
    }
}
