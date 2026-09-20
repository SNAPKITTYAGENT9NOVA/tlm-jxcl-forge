// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The resource-creation pipeline: `AUTHORIZE -> VALIDATE -> PLAN ->
//! APPLY -> VERIFY -> AUDIT`.
//!
//! (`REQUEST` and `AUTHENTICATE`, from the roadmap's full state
//! machine, happen before this crate is called: this pipeline takes
//! an already-authenticated [`Principal`] as input. `COMMIT` happens
//! after: this crate has no storage of its own, so persisting the
//! provisioned resource is the caller's job -- see
//! [`release_reservation`] for what to call if that persistence step
//! fails.)
//!
//! ## The rollback invariant
//!
//! If [`provision`] fails at `VALIDATE`, `PLAN`, or `APPLY`, every
//! reservation an earlier stage made (a quota unit, an availability
//! zone's usage count) is released before the error is returned. A
//! caller never has to guess whether a failed `provision()` call left
//! partial state behind -- it never does.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_identity::Principal;
use cloud_policy::Policy;
use cloud_quota::QuotaTracker;
use cloud_region::RegionRegistry;
use cloud_resource::Resource;
use cloud_types::{AccountId, AzId, RegionId, ResourceId, ResourceType, Timestamp};
use std::collections::BTreeMap;

/// A request to provision one new resource.
pub struct ProvisionRequest<T> {
    pub id: ResourceId,
    pub resource_type: ResourceType,
    pub account: AccountId,
    pub principal: Principal,
    pub action: String,
    pub region: RegionId,
    pub created_at: Timestamp,
    pub payload: T,
    pub tags: Vec<(String, String)>,
}

/// The shared state the pipeline reads from and writes to. Borrowed,
/// not owned, since the caller (typically `cloud-control-plane`) owns
/// all of it across many provisioning calls.
pub struct ProvisionContext<'a> {
    pub policy: &'a Policy,
    pub quota: &'a mut QuotaTracker,
    /// Which `cloud-quota` resource category this request consumes
    /// one unit of (e.g. `"instances"`).
    pub quota_resource: &'a str,
    pub region_registry: &'a RegionRegistry,
    pub az_usage: &'a mut BTreeMap<AzId, u64>,
    pub events: &'a mut EventLog,
}

/// The result of a successful provision: the constructed resource,
/// and the availability zone `PLAN` chose for it.
#[derive(Debug)]
pub struct Provisioned<T> {
    pub resource: Resource<T>,
    pub az: AzId,
}

/// Runs the `AUTHORIZE -> VALIDATE -> PLAN -> APPLY -> VERIFY ->
/// AUDIT` pipeline for `request` against `ctx`. See the module's own
/// doc comment for the rollback guarantee on failure.
pub fn provision<T>(
    request: ProvisionRequest<T>,
    ctx: &mut ProvisionContext,
) -> Result<Provisioned<T>, CloudError> {
    // AUTHORIZE
    ctx.policy
        .authorize(&request.principal, &request.action, request.id.as_str())?;

    // VALIDATE
    ctx.quota.try_reserve(ctx.quota_resource, 1)?;

    // PLAN
    let az = match cloud_scheduler::place_least_loaded(
        ctx.region_registry,
        &request.region,
        ctx.az_usage,
    ) {
        Ok(az) => az,
        Err(e) => {
            ctx.quota.release(ctx.quota_resource, 1);
            return Err(e);
        }
    };

    // APPLY
    let mut resource = Resource::new(
        request.id,
        request.resource_type,
        Some(request.region),
        request.account,
        request.created_at,
        request.payload,
    );
    for (key, value) in &request.tags {
        if let Err(e) = resource.set_tag(key.clone(), value.clone(), request.created_at) {
            ctx.quota.release(ctx.quota_resource, 1);
            release_az_usage(ctx.az_usage, &az);
            return Err(e);
        }
    }

    // VERIFY -- a defense-in-depth check that every requested tag
    // actually landed with the right value. Given APPLY's own
    // per-tag error handling above, this should be unreachable in
    // practice; it exists so a future change to APPLY that breaks
    // this guarantee fails loudly here rather than silently.
    for (key, value) in &request.tags {
        if resource.tags().get(key) != Some(value.as_str()) {
            ctx.quota.release(ctx.quota_resource, 1);
            release_az_usage(ctx.az_usage, &az);
            return Err(CloudError::InvalidFormat {
                what: "provisioned resource",
                value: resource.id().to_string(),
                reason: format!("tag {key:?} did not verify after apply"),
            });
        }
    }

    // AUDIT
    ctx.events.append(
        resource.id().clone(),
        "resource.provisioned",
        request.created_at,
        format!(
            "account={} type={} az={}",
            resource.account(),
            resource.resource_type(),
            az
        ),
    );

    Ok(Provisioned { resource, az })
}

/// Releases the quota unit and availability-zone usage count that a
/// successful [`provision`] call reserved, for a caller whose own
/// downstream commit step (e.g. inserting into a resource store)
/// failed after `provision` already returned `Ok`. `provision` itself
/// never leaves a reservation behind on its own failure paths -- this
/// function is only for a failure that happens *after* `provision`
/// succeeded.
pub fn release_reservation(
    quota: &mut QuotaTracker,
    quota_resource: &str,
    az_usage: &mut BTreeMap<AzId, u64>,
    az: &AzId,
) {
    quota.release(quota_resource, 1);
    release_az_usage(az_usage, az);
}

fn release_az_usage(usage: &mut BTreeMap<AzId, u64>, az: &AzId) {
    if let Some(count) = usage.get_mut(az) {
        *count = count.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_lifecycle::Lifecycle;
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

    fn deny_all_policy() -> Policy {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Deny,
            principals: PrincipalMatcher::Any,
            actions: vec!["*".to_string()],
            resources: vec!["*".to_string()],
        });
        policy
    }

    fn registry_with(region: &str, az_count: u32) -> RegionRegistry {
        let mut reg = RegionRegistry::new();
        reg.register(RegionId::new(region).unwrap(), az_count)
            .unwrap();
        reg
    }

    fn request(tags: Vec<(&str, &str)>) -> ProvisionRequest<u32> {
        ProvisionRequest {
            id: ResourceId::new("i-1").unwrap(),
            resource_type: ResourceType::new("compute-instance").unwrap(),
            account: AccountId::new("000000000001").unwrap(),
            principal: Principal::User(ResourceId::new("alice").unwrap()),
            action: "compute:create".to_string(),
            region: RegionId::new("us-west-1").unwrap(),
            created_at: Timestamp::from_millis(1_000),
            payload: 42,
            tags: tags
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn a_successful_provision_reserves_quota_places_and_records_an_event() {
        let policy = allow_all_policy();
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 5);
        let registry = registry_with("us-west-1", 2);
        let mut az_usage = BTreeMap::new();
        let mut events = EventLog::new();

        let outcome = provision(
            request(vec![("env", "prod")]),
            &mut ProvisionContext {
                policy: &policy,
                quota: &mut quota,
                quota_resource: "instances",
                region_registry: &registry,
                az_usage: &mut az_usage,
                events: &mut events,
            },
        )
        .unwrap();

        assert_eq!(outcome.resource.lifecycle(), Lifecycle::Creating);
        assert_eq!(outcome.resource.tags().get("env"), Some("prod"));
        assert_eq!(quota.usage("instances"), 1);
        assert_eq!(az_usage[&outcome.az], 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events.events()[0].kind, "resource.provisioned");
    }

    #[test]
    fn authorize_failure_reserves_nothing_and_records_no_event() {
        let policy = deny_all_policy();
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 5);
        let registry = registry_with("us-west-1", 2);
        let mut az_usage = BTreeMap::new();
        let mut events = EventLog::new();

        let err = provision(
            request(vec![]),
            &mut ProvisionContext {
                policy: &policy,
                quota: &mut quota,
                quota_resource: "instances",
                region_registry: &registry,
                az_usage: &mut az_usage,
                events: &mut events,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::PolicyDenied { .. }));
        assert_eq!(quota.usage("instances"), 0);
        assert!(az_usage.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn quota_failure_leaves_no_placement_and_no_event() {
        let policy = allow_all_policy();
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 0); // no room at all
        let registry = registry_with("us-west-1", 2);
        let mut az_usage = BTreeMap::new();
        let mut events = EventLog::new();

        let err = provision(
            request(vec![]),
            &mut ProvisionContext {
                policy: &policy,
                quota: &mut quota,
                quota_resource: "instances",
                region_registry: &registry,
                az_usage: &mut az_usage,
                events: &mut events,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert!(az_usage.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn plan_failure_rolls_back_the_quota_reservation() {
        let policy = allow_all_policy();
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 5);
        let registry = RegionRegistry::new(); // region never registered
        let mut az_usage = BTreeMap::new();
        let mut events = EventLog::new();

        let err = provision(
            request(vec![]),
            &mut ProvisionContext {
                policy: &policy,
                quota: &mut quota,
                quota_resource: "instances",
                region_registry: &registry,
                az_usage: &mut az_usage,
                events: &mut events,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::NotFound { .. }));
        assert_eq!(
            quota.usage("instances"),
            0,
            "the quota reservation from VALIDATE must be rolled back"
        );
        assert!(events.is_empty());
    }

    #[test]
    fn apply_failure_rolls_back_both_quota_and_placement() {
        let policy = allow_all_policy();
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 5);
        let registry = registry_with("us-west-1", 1);
        let mut az_usage = BTreeMap::new();
        let mut events = EventLog::new();

        // A reserved-prefix tag key is rejected by cloud-tags, which
        // APPLY surfaces as a failure.
        let err = provision(
            request(vec![("jxcl:reserved", "x")]),
            &mut ProvisionContext {
                policy: &policy,
                quota: &mut quota,
                quota_resource: "instances",
                region_registry: &registry,
                az_usage: &mut az_usage,
                events: &mut events,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::InvalidFormat { .. }));
        assert_eq!(
            quota.usage("instances"),
            0,
            "APPLY failure must roll back the VALIDATE-stage quota reservation"
        );
        assert_eq!(
            az_usage[&AzId::new("us-west-1a").unwrap()],
            0,
            "APPLY failure must roll back the PLAN-stage placement"
        );
        assert!(events.is_empty());
    }

    #[test]
    fn release_reservation_undoes_a_successful_provisions_effects() {
        let policy = allow_all_policy();
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 5);
        let registry = registry_with("us-west-1", 1);
        let mut az_usage = BTreeMap::new();
        let mut events = EventLog::new();

        let outcome = provision(
            request(vec![]),
            &mut ProvisionContext {
                policy: &policy,
                quota: &mut quota,
                quota_resource: "instances",
                region_registry: &registry,
                az_usage: &mut az_usage,
                events: &mut events,
            },
        )
        .unwrap();
        assert_eq!(quota.usage("instances"), 1);
        assert_eq!(az_usage[&outcome.az], 1);

        // Simulate the caller's own COMMIT (store insert) failing.
        release_reservation(&mut quota, "instances", &mut az_usage, &outcome.az);

        assert_eq!(quota.usage("instances"), 0);
        assert_eq!(az_usage[&outcome.az], 0);
    }
}
