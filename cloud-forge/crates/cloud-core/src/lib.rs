// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A thin facade re-exporting `cloud-forge`'s Phase 1 primitive
//! kernel under one crate, so a caller composing several primitives
//! doesn't need to depend on all eleven kernel crates by hand. This
//! crate owns no new logic of its own -- see each re-exported type's
//! home crate for its actual implementation and tests.
//!
//! Its own test suite is a single integration test showing the
//! primitives actually compose into something resembling a real
//! provisioning flow -- the whole point of Phase 1, per this
//! workspace's non-negotiable rule (see
//! `cloud-forge/docs/CLOUD_ARCHITECTURE.md`).
#![forbid(unsafe_code)]

pub use cloud_account::{Account, AccountRegistry};
pub use cloud_errors::{CloudError, CloudResult};
pub use cloud_events::{Event, EventLog};
pub use cloud_identity::Principal;
pub use cloud_lifecycle::Lifecycle;
pub use cloud_policy::{Decision, Effect, Policy, PrincipalMatcher, Statement};
pub use cloud_quota::QuotaTracker;
pub use cloud_region::{RegionInfo, RegionRegistry};
pub use cloud_resource::Resource;
pub use cloud_tags::Tags;
pub use cloud_types::{AccountId, Arn, AzId, RegionId, ResourceId, ResourceType, Timestamp};

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal service-specific payload, standing in for a future
    /// `InstanceSpec`/`BucketSpec`/etc.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DemoInstanceSpec {
        cpu_count: u32,
    }

    #[test]
    fn provisioning_a_resource_composes_every_phase_1_primitive() {
        // 1. Register a region with two availability zones.
        let mut regions = RegionRegistry::new();
        let region = RegionId::new("us-west-1").unwrap();
        regions.register(region.clone(), 2).unwrap();
        let az = AzId::new("us-west-1a").unwrap();
        assert!(regions.contains_az(&az));

        // 2. Register an owning account.
        let mut accounts = AccountRegistry::new();
        let account_id = AccountId::new("000000000001").unwrap();
        accounts
            .register(Account::new(account_id.clone(), "demo-account", Timestamp::EPOCH).unwrap())
            .unwrap();

        // 3. A policy allowing the account's admin role to manage
        //    compute resources, but explicitly denying termination --
        //    demonstrating deny-dominates even though a broader Allow
        //    also matches.
        let admin = Principal::Role(ResourceId::new("admin").unwrap());
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::OneOf(vec![admin.clone()]),
            actions: vec!["compute:*".to_string()],
            resources: vec!["*".to_string()],
        });
        policy.add_statement(Statement {
            effect: Effect::Deny,
            principals: PrincipalMatcher::Any,
            actions: vec!["compute:terminate".to_string()],
            resources: vec!["*".to_string()],
        });
        policy
            .authorize(&admin, "compute:describe", "i-1")
            .expect("admin may describe");
        assert!(
            policy
                .authorize(&admin, "compute:terminate", "i-1")
                .is_err(),
            "the explicit deny must win even for the admin"
        );

        // 4. Quota: this account may create at most 2 instances.
        let mut quota = QuotaTracker::new();
        quota.set_limit("instances", 2);
        quota.try_reserve("instances", 1).unwrap();

        // 5. Create the resource itself, tag it, and move it through
        //    its lifecycle -- checking the version invariant holds
        //    across every step.
        let mut instance = Resource::new(
            ResourceId::new("i-000001").unwrap(),
            ResourceType::new("compute-instance").unwrap(),
            Some(region.clone()),
            account_id.clone(),
            Timestamp::from_millis(1_000),
            DemoInstanceSpec { cpu_count: 4 },
        );
        assert_eq!(instance.version(), 1);
        assert_eq!(instance.lifecycle(), Lifecycle::Creating);

        instance
            .set_tag("env", "demo", Timestamp::from_millis(1_100))
            .unwrap();
        instance
            .transition_lifecycle(Lifecycle::Active, Timestamp::from_millis(1_200))
            .unwrap();
        assert_eq!(instance.lifecycle(), Lifecycle::Active);
        assert_eq!(instance.version(), 3); // 1 (created) + tag + transition

        // 6. This system's internal ARN for the resource, and an
        //    event recording that it was created.
        let arn = Arn::new(
            "core",
            "compute",
            Some(region),
            Some(account_id),
            instance.id().clone(),
        )
        .unwrap();
        assert_eq!(
            arn.to_string(),
            "jxcl:cloud:core:compute:us-west-1:000000000001:i-000001"
        );

        let mut events = EventLog::new();
        let event_id = events.append(
            instance.id().clone(),
            "resource.created",
            Timestamp::from_millis(1_000),
            arn.to_string(),
        );
        assert_eq!(event_id, 1);
        assert_eq!(events.len(), 1);

        // 7. A second instance would exceed the quota.
        assert!(quota.try_reserve("instances", 2).is_err());
        assert_eq!(
            quota.usage("instances"),
            1,
            "the rejected reservation must not have committed"
        );
    }
}
