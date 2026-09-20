// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `DatabaseService`: the third real composed service in this
//! workspace, following the shape `cloud-compute` (Phase 8) and
//! `cloud-storage` (Phase 9) both established -- compose already-real
//! primitives, introduce none.
//!
//! A database is created with a chosen [`ConsistencyLevel`] (Phase 6)
//! and its own [`MigrationLedger`] (Phase 6), starting at schema
//! version 0. [`DatabaseService::create_snapshot`] and
//! [`DatabaseService::expire_snapshots`] give [`RetentionPolicy`]
//! (Phase 6) its first real, stateful use anywhere in this workspace:
//! Phase 6 only computed *which* snapshots a policy would allow
//! deleting, given a list; this service is the first thing that
//! actually keeps that list and acts on the answer, resolving
//! `cloud-storage`'s own Phase 9 deferral note ("wiring \[`cloud-retention`\]
//! in is future work once a snapshot concept exists to attach them to").
#![forbid(unsafe_code)]

use cloud_account::AccountRegistry;
use cloud_consistency::ConsistencyLevel;
use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_identity::Principal;
use cloud_lifecycle::Lifecycle;
use cloud_migration::MigrationLedger;
use cloud_policy::Policy;
use cloud_quota::QuotaTracker;
use cloud_region::RegionRegistry;
use cloud_resource::Resource;
use cloud_resource_registry::ResourceRegistry;
use cloud_retention::RetentionPolicy;
use cloud_types::{AccountId, Arn, RegionId, ResourceId, ResourceType, Timestamp};
use std::collections::BTreeMap;

/// A database's service-specific payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseSpec {
    pub consistency: ConsistencyLevel,
    pub retention: RetentionPolicy,
}

/// A request to create one new database.
pub struct CreateDatabaseRequest {
    pub id: ResourceId,
    pub account: AccountId,
    pub region: RegionId,
    pub principal: Principal,
    pub consistency: ConsistencyLevel,
    pub retention: RetentionPolicy,
    pub created_at: Timestamp,
}

/// The database service: holds every registry `create_database`/
/// `delete_database` needs, and the store of databases it has created.
pub struct DatabaseService {
    partition: String,
    service: String,
    accounts: AccountRegistry,
    regions: RegionRegistry,
    policy: Policy,
    quota: QuotaTracker,
    events: EventLog,
    names: ResourceRegistry,
    databases: BTreeMap<ResourceId, Resource<DatabaseSpec>>,
    migrations: BTreeMap<ResourceId, MigrationLedger>,
    snapshots: BTreeMap<ResourceId, Vec<(ResourceId, Timestamp)>>,
}

impl DatabaseService {
    pub fn new(policy: Policy, partition: impl Into<String>, service: impl Into<String>) -> Self {
        DatabaseService {
            partition: partition.into(),
            service: service.into(),
            accounts: AccountRegistry::new(),
            regions: RegionRegistry::new(),
            policy,
            quota: QuotaTracker::new(),
            events: EventLog::new(),
            names: ResourceRegistry::new(),
            databases: BTreeMap::new(),
            migrations: BTreeMap::new(),
            snapshots: BTreeMap::new(),
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

    pub fn get(&self, id: &ResourceId) -> Option<&Resource<DatabaseSpec>> {
        self.databases.get(id)
    }

    pub fn list(&self) -> impl Iterator<Item = &Resource<DatabaseSpec>> {
        self.databases.values()
    }

    pub fn resolve_arn(&self, arn: &Arn) -> Option<&Resource<DatabaseSpec>> {
        self.names
            .resolve(arn)
            .and_then(|id| self.databases.get(id))
    }

    pub fn arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.names.arn_of(id)
    }

    pub fn current_schema_version(&self, id: &ResourceId) -> Option<u32> {
        self.migrations
            .get(id)
            .map(MigrationLedger::current_version)
    }

    pub fn snapshots(&self, id: &ResourceId) -> Option<&[(ResourceId, Timestamp)]> {
        self.snapshots.get(id).map(Vec::as_slice)
    }

    /// Creates a new database: validates the account and region exist,
    /// authorizes the request, reserves a `"databases"` quota unit,
    /// constructs and names the resource, and starts it with an empty
    /// `MigrationLedger` (schema version 0) and no snapshots. Any
    /// failure after the quota was reserved releases it.
    pub fn create_database(
        &mut self,
        request: CreateDatabaseRequest,
    ) -> Result<&Resource<DatabaseSpec>, CloudError> {
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
        if self.databases.contains_key(&request.id) {
            return Err(CloudError::Conflict {
                what: "database",
                id: request.id.to_string(),
            });
        }

        self.policy
            .authorize(&request.principal, "database:create", request.id.as_str())?;

        self.quota.try_reserve("databases", 1)?;

        let resource_type = ResourceType::new("database-instance").expect("valid literal");
        let resource = Resource::new(
            request.id.clone(),
            resource_type,
            Some(request.region.clone()),
            request.account.clone(),
            request.created_at,
            DatabaseSpec {
                consistency: request.consistency,
                retention: request.retention,
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
                self.quota.release("databases", 1);
                return Err(e);
            }
        };
        if let Err(e) = self.names.register(&arn, request.id.clone()) {
            self.quota.release("databases", 1);
            return Err(e);
        }

        self.events.append(
            request.id.clone(),
            "database.created",
            request.created_at,
            format!("account={}", resource.account()),
        );

        self.databases.insert(request.id.clone(), resource);
        self.migrations
            .insert(request.id.clone(), MigrationLedger::new());
        self.snapshots.insert(request.id.clone(), Vec::new());
        Ok(self
            .databases
            .get(&request.id)
            .expect("just inserted under this exact id"))
    }

    /// Applies schema migration `version` to `id`'s `MigrationLedger`.
    /// Rejected (leaving the ledger unchanged) unless `version` is
    /// exactly one more than the database's current schema version.
    pub fn apply_migration(&mut self, id: &ResourceId, version: u32) -> Result<u32, CloudError> {
        let ledger = self
            .migrations
            .get_mut(id)
            .ok_or_else(|| CloudError::NotFound {
                what: "database",
                id: id.to_string(),
            })?;
        ledger.apply(version)?;
        Ok(ledger.current_version())
    }

    /// Records a new snapshot for `id`, created at `now`. Rejects a
    /// second snapshot under an already-used `snapshot_id` for the
    /// same database.
    pub fn create_snapshot(
        &mut self,
        id: &ResourceId,
        snapshot_id: ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        let snapshots = self
            .snapshots
            .get_mut(id)
            .ok_or_else(|| CloudError::NotFound {
                what: "database",
                id: id.to_string(),
            })?;
        if snapshots
            .iter()
            .any(|(existing, _)| *existing == snapshot_id)
        {
            return Err(CloudError::Conflict {
                what: "snapshot",
                id: snapshot_id.to_string(),
            });
        }
        snapshots.push((snapshot_id, now));
        Ok(())
    }

    /// Applies `id`'s own [`RetentionPolicy`] against its recorded
    /// snapshots at `now`, actually removing every eligible one from
    /// the store and returning their ids -- the stateful half of what
    /// `cloud-retention` (Phase 6) only computes.
    pub fn expire_snapshots(
        &mut self,
        id: &ResourceId,
        now: Timestamp,
    ) -> Result<Vec<ResourceId>, CloudError> {
        let resource = self.databases.get(id).ok_or_else(|| CloudError::NotFound {
            what: "database",
            id: id.to_string(),
        })?;
        let retention = resource.payload().retention;
        let snapshots = self
            .snapshots
            .get_mut(id)
            .expect("database and snapshot stores are always kept in sync");
        let expired = retention.eligible_for_deletion(now, snapshots);
        snapshots.retain(|(existing, _)| !expired.contains(existing));
        Ok(expired)
    }

    /// Deletes a database: refuses while any snapshot is still
    /// recorded (call [`Self::expire_snapshots`], or let every
    /// snapshot naturally age out, first), reconciles its lifecycle to
    /// `Deleted`, and releases its `"databases"` quota unit.
    pub fn delete_database(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        let snapshots = self.snapshots.get(id).ok_or_else(|| CloudError::NotFound {
            what: "database",
            id: id.to_string(),
        })?;
        if !snapshots.is_empty() {
            return Err(CloudError::Conflict {
                what: "database",
                id: id.to_string(),
            });
        }

        let resource = self
            .databases
            .get_mut(id)
            .expect("database and snapshot stores are always kept in sync");
        let plan = cloud_reconciler::reconcile(resource.lifecycle(), Lifecycle::Deleted)?;
        for step in plan {
            resource
                .transition_lifecycle(step, now)
                .expect("cloud-reconciler only ever returns already-valid single steps");
        }

        self.quota.release("databases", 1);
        self.events
            .append(id.clone(), "database.deleted", now, "".to_string());
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

    fn setup() -> DatabaseService {
        let mut svc = DatabaseService::new(allow_all_policy(), "core", "database");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.set_quota_limit("databases", 5);
        svc
    }

    fn create_request(id: &str) -> CreateDatabaseRequest {
        CreateDatabaseRequest {
            id: ResourceId::new(id).unwrap(),
            account: account_id(),
            region: region_id(),
            principal: Principal::User(ResourceId::new("alice").unwrap()),
            consistency: ConsistencyLevel::Strong,
            retention: RetentionPolicy::new(1000, 1).unwrap(),
            created_at: Timestamp::from_millis(1000),
        }
    }

    #[test]
    fn create_rejects_an_unregistered_account() {
        let mut svc = setup();
        let mut req = create_request("db-1");
        req.account = AccountId::new("999999999999").unwrap();
        let err = svc.create_database(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_rejects_an_unregistered_region() {
        let mut svc = setup();
        let mut req = create_request("db-1");
        req.region = RegionId::new("eu-central-1").unwrap();
        let err = svc.create_database(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_rejects_a_duplicate_id() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let err = svc.create_database(create_request("db-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn a_successful_create_reserves_quota_and_starts_at_schema_zero() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Creating);
        assert_eq!(
            svc.get(&id).unwrap().payload().consistency,
            ConsistencyLevel::Strong
        );
        assert_eq!(svc.current_schema_version(&id), Some(0));
        assert_eq!(svc.quota.usage("databases"), 1);
    }

    #[test]
    fn create_registers_a_resolvable_arn() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        let arn = svc.arn_of(&id).unwrap();
        assert_eq!(svc.resolve_arn(&arn).unwrap().id(), &id);
    }

    #[test]
    fn exhausting_quota_fails_create_and_reserves_nothing() {
        let mut svc = setup();
        svc.set_quota_limit("databases", 0);
        let err = svc.create_database(create_request("db-1")).unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(svc.quota.usage("databases"), 0);
    }

    #[test]
    fn apply_migration_enforces_sequential_versions() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        assert_eq!(svc.apply_migration(&id, 1).unwrap(), 1);
        assert_eq!(svc.apply_migration(&id, 2).unwrap(), 2);
        let err = svc.apply_migration(&id, 4).unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert_eq!(svc.current_schema_version(&id), Some(2));
    }

    #[test]
    fn apply_migration_of_an_unknown_database_is_not_found() {
        let mut svc = setup();
        let err = svc
            .apply_migration(&ResourceId::new("db-ghost").unwrap(), 1)
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_snapshot_then_it_appears_in_the_snapshot_list() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        svc.create_snapshot(
            &id,
            ResourceId::new("snap-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        assert_eq!(svc.snapshots(&id).unwrap().len(), 1);
    }

    #[test]
    fn create_snapshot_rejects_a_duplicate_snapshot_id() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        let snap = ResourceId::new("snap-1").unwrap();
        svc.create_snapshot(&id, snap.clone(), Timestamp::from_millis(2000))
            .unwrap();
        let err = svc
            .create_snapshot(&id, snap, Timestamp::from_millis(3000))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn create_snapshot_of_an_unknown_database_is_not_found() {
        let mut svc = setup();
        let err = svc
            .create_snapshot(
                &ResourceId::new("db-ghost").unwrap(),
                ResourceId::new("snap-1").unwrap(),
                Timestamp::from_millis(1000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn expire_snapshots_removes_only_expired_ones_beyond_the_floor() {
        let mut svc = setup();
        let mut req = create_request("db-1");
        req.retention = RetentionPolicy::new(1000, 1).unwrap();
        svc.create_database(req).unwrap();
        let id = ResourceId::new("db-1").unwrap();

        svc.create_snapshot(
            &id,
            ResourceId::new("snap-old").unwrap(),
            Timestamp::from_millis(0),
        )
        .unwrap();
        svc.create_snapshot(
            &id,
            ResourceId::new("snap-new").unwrap(),
            Timestamp::from_millis(100),
        )
        .unwrap();

        let expired = svc
            .expire_snapshots(&id, Timestamp::from_millis(1100))
            .unwrap();
        assert_eq!(expired, vec![ResourceId::new("snap-old").unwrap()]);
        assert_eq!(svc.snapshots(&id).unwrap().len(), 1);
    }

    #[test]
    fn delete_database_refuses_while_snapshots_remain() {
        let mut svc = setup();
        svc.create_database(create_request("db-1")).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        svc.create_snapshot(
            &id,
            ResourceId::new("snap-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();

        let err = svc
            .delete_database(&id, Timestamp::from_millis(3000))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        assert_eq!(
            svc.quota.usage("databases"),
            1,
            "a refused delete must not release quota"
        );
    }

    #[test]
    fn delete_database_succeeds_once_snapshots_are_gone_and_releases_quota() {
        let mut svc = setup();
        let mut req = create_request("db-1");
        req.retention = RetentionPolicy::new(1000, 0).unwrap();
        svc.create_database(req).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        svc.create_snapshot(
            &id,
            ResourceId::new("snap-1").unwrap(),
            Timestamp::from_millis(0),
        )
        .unwrap();
        svc.expire_snapshots(&id, Timestamp::from_millis(1000))
            .unwrap();

        svc.delete_database(&id, Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(svc.get(&id).unwrap().lifecycle(), Lifecycle::Deleted);
        assert_eq!(svc.quota.usage("databases"), 0);
    }

    #[test]
    fn delete_database_of_an_unknown_database_is_not_found() {
        let mut svc = setup();
        let err = svc
            .delete_database(
                &ResourceId::new("db-ghost").unwrap(),
                Timestamp::from_millis(1000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn list_includes_deleted_databases() {
        let mut svc = setup();
        let mut req = create_request("db-1");
        req.retention = RetentionPolicy::new(1000, 0).unwrap();
        svc.create_database(req).unwrap();
        let id = ResourceId::new("db-1").unwrap();
        svc.delete_database(&id, Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(svc.list().count(), 1);
    }
}
