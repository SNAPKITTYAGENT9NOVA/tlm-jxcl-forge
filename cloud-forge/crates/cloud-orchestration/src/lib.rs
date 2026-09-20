// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Cross-service composition between `cloud-compute` and `cloud-storage`
//! -- the first phase in this workspace to compose two already-real
//! **services** together, rather than composing primitives into a
//! service.
//!
//! `cloud-storage`'s own Phase 9 doc comment named exactly this as
//! deferred: "attaching a volume moves only its own `AttachmentState`;
//! whether the id it's attached to names a real `cloud-compute`
//! instance is an orchestration concern for whatever future layer
//! calls both services." This crate is that layer. It holds no state
//! of its own and introduces no new state machine -- [`attach_volume`]
//! and [`detach_volume`] are free functions that read one service and
//! mutate the other, exactly the shape `cloud-provisioner` (Phase 2)
//! already established for a pipeline with nothing to own between
//! calls.
//!
//! `attach_volume` validates the instance *before* touching the
//! volume's attachment state at all: it refuses an unknown instance,
//! and refuses an instance that is `Terminating` or `Terminated`
//! (attaching storage to something already being torn down, or gone,
//! makes no sense), leaving the volume's `AttachmentState` completely
//! untouched in either case. Once that check passes, both attachment
//! transitions (`Detached -> Attaching -> Attached`) are driven in
//! sequence. Neither can fail once started: `cloud-attachment`'s own
//! transition table makes `Attaching -> Attached` unconditionally valid
//! from a state this function itself just placed the volume into, so
//! there is no partial-attach state to roll back from, unlike the
//! multi-stage pipelines in earlier phases that reserve real resources
//! before every step.
//!
//! `detach_volume` does **not** check the instance's state at all --
//! detaching is presumed always safe, mirroring how a real block-storage
//! service can force-detach a volume even from an instance that no
//! longer exists. The concern there is releasing the volume, not the
//! instance's own health.
//!
//! ## Phase 15: a second composition, this time with a real rollback
//!
//! [`transition_runtime_and_notify`] and [`terminate_and_notify`]
//! compose `cloud-compute` with `cloud-messaging`, resolving
//! `cloud-messaging`'s own Phase 11 closing note: "what remains
//! deferred is composing these services *together* (e.g. a compute
//! instance's logs delivered through a queue)." Both functions enqueue
//! a notification message *before* attempting the compute mutation,
//! then roll the message back with `MessagingService::delete_message`
//! if that mutation fails -- unlike [`attach_volume`]/[`detach_volume`],
//! which needed no rollback at all (their second step was always
//! unconditionally valid once the first succeeded), a `RuntimeState`
//! transition genuinely can fail (an invalid jump, an unknown
//! instance), and `RuntimeState` transitions are not generally
//! reversible, so the only correct order is: reserve the notification
//! first, then attempt the mutation, then undo the reservation if the
//! mutation didn't happen. This is the first rollback in this
//! workspace that spans two independent services rather than one.
//!
//! ## Phase 16: a third composition, database operations with notifications
//!
//! [`apply_migration_and_notify`] and [`delete_database_and_notify`]
//! compose `cloud-database` with `cloud-messaging`, following the same
//! enqueue-first, rollback-on-failure pattern as Phase 15. Database
//! operations like schema migration and deletion can fail or be
//! non-reversible (unlike [`attach_volume`]'s always-valid second step),
//! so both functions enqueue the notification message first, then attempt
//! the database mutation, then delete the message if the mutation fails.
//! This is the second composition in this crate spanning two independent
//! services rather than one.
#![forbid(unsafe_code)]

use cloud_attachment::AttachmentState;
use cloud_compute::ComputeService;
use cloud_database::DatabaseService;
use cloud_errors::CloudError;
use cloud_messaging::MessagingService;
use cloud_runtime::RuntimeState;
use cloud_storage::StorageService;
use cloud_types::{ResourceId, Timestamp};

/// Attaches `volume_id` (in `storage`) to `instance_id` (in `compute`).
/// Refuses unless `instance_id` names a real, non-terminating,
/// non-terminated instance -- checked before any attachment state
/// changes at all.
pub fn attach_volume(
    compute: &ComputeService,
    storage: &mut StorageService,
    instance_id: &ResourceId,
    volume_id: &ResourceId,
) -> Result<AttachmentState, CloudError> {
    let runtime = compute
        .runtime_state(instance_id)
        .ok_or_else(|| CloudError::NotFound {
            what: "instance",
            id: instance_id.to_string(),
        })?;
    if matches!(
        runtime,
        RuntimeState::Terminating | RuntimeState::Terminated
    ) {
        return Err(CloudError::Conflict {
            what: "instance",
            id: instance_id.to_string(),
        });
    }

    storage.transition_attachment(volume_id, AttachmentState::Attaching)?;
    storage.transition_attachment(volume_id, AttachmentState::Attached)
}

/// Detaches `volume_id` (in `storage`). Does not consult `compute` at
/// all -- detaching must always be possible regardless of whatever
/// instance the volume was attached to.
pub fn detach_volume(
    storage: &mut StorageService,
    volume_id: &ResourceId,
) -> Result<AttachmentState, CloudError> {
    storage.transition_attachment(volume_id, AttachmentState::Detaching)?;
    storage.transition_attachment(volume_id, AttachmentState::Detached)
}

/// Transitions `instance_id`'s runtime state and enqueues
/// `message_id` onto `queue_id` to notify of it, as one unit: the
/// message is enqueued first, and rolled back (via `delete_message`)
/// if the runtime transition then fails, so a failed transition never
/// leaves a stray notification behind.
pub fn transition_runtime_and_notify(
    compute: &mut ComputeService,
    messaging: &mut MessagingService,
    instance_id: &ResourceId,
    to: RuntimeState,
    queue_id: &ResourceId,
    message_id: ResourceId,
    now: Timestamp,
) -> Result<RuntimeState, CloudError> {
    messaging.enqueue(queue_id, message_id.clone(), now)?;
    match compute.transition_runtime(instance_id, to) {
        Ok(state) => Ok(state),
        Err(e) => {
            messaging
                .delete_message(queue_id, &message_id, now)
                .expect("the message just enqueued above must still be present to delete");
            Err(e)
        }
    }
}

/// Terminates `instance_id` and enqueues `message_id` onto `queue_id`
/// to notify of it, with the same enqueue-first, roll-back-on-failure
/// discipline as [`transition_runtime_and_notify`].
pub fn terminate_and_notify(
    compute: &mut ComputeService,
    messaging: &mut MessagingService,
    instance_id: &ResourceId,
    queue_id: &ResourceId,
    message_id: ResourceId,
    now: Timestamp,
) -> Result<(), CloudError> {
    messaging.enqueue(queue_id, message_id.clone(), now)?;
    match compute.terminate(instance_id, now) {
        Ok(()) => Ok(()),
        Err(e) => {
            messaging
                .delete_message(queue_id, &message_id, now)
                .expect("the message just enqueued above must still be present to delete");
            Err(e)
        }
    }
}

/// Applies a schema migration to `database_id` and enqueues `message_id`
/// onto `queue_id` to notify of it, with the same enqueue-first,
/// roll-back-on-failure discipline as Phase 15's compositions.
pub fn apply_migration_and_notify(
    database: &mut DatabaseService,
    messaging: &mut MessagingService,
    database_id: &ResourceId,
    version: u32,
    queue_id: &ResourceId,
    message_id: ResourceId,
    now: Timestamp,
) -> Result<u32, CloudError> {
    messaging.enqueue(queue_id, message_id.clone(), now)?;
    match database.apply_migration(database_id, version) {
        Ok(v) => Ok(v),
        Err(e) => {
            messaging
                .delete_message(queue_id, &message_id, now)
                .expect("the message just enqueued above must still be present to delete");
            Err(e)
        }
    }
}

/// Deletes `database_id` and enqueues `message_id` onto `queue_id` to
/// notify of it, with the same enqueue-first, roll-back-on-failure
/// discipline as [`apply_migration_and_notify`].
pub fn delete_database_and_notify(
    database: &mut DatabaseService,
    messaging: &mut MessagingService,
    database_id: &ResourceId,
    queue_id: &ResourceId,
    message_id: ResourceId,
    now: Timestamp,
) -> Result<(), CloudError> {
    messaging.enqueue(queue_id, message_id.clone(), now)?;
    match database.delete_database(database_id, now) {
        Ok(()) => Ok(()),
        Err(e) => {
            messaging
                .delete_message(queue_id, &message_id, now)
                .expect("the message just enqueued above must still be present to delete");
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_account::Account;
    use cloud_capacity::Capacity;
    use cloud_compute::LaunchRequest;
    use cloud_consistency::ConsistencyLevel;
    use cloud_database::CreateDatabaseRequest;
    use cloud_delivery::DeliverySemantics;
    use cloud_identity::Principal;
    use cloud_image::Architecture;
    use cloud_lifecycle::Lifecycle;
    use cloud_messaging::CreateQueueRequest;
    use cloud_policy::{Effect, Policy, PrincipalMatcher, Statement};
    use cloud_redundancy::RedundancyScheme;
    use cloud_retention::RetentionPolicy;
    use cloud_storage::CreateVolumeRequest;
    use cloud_types::{AccountId, AzId, RegionId, Timestamp};

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

    fn principal() -> Principal {
        Principal::User(ResourceId::new("alice").unwrap())
    }

    fn setup_compute() -> ComputeService {
        let mut svc = ComputeService::new(allow_all_policy(), "core", "compute");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.register_az_capacity(AzId::new("us-west-1a").unwrap(), Capacity::new(16, 65536))
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

    fn setup_storage() -> StorageService {
        let mut svc = StorageService::new(allow_all_policy(), "core", "storage");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.set_quota_limit("storage_bytes", 1_000_000);
        svc
    }

    fn launch(compute: &mut ComputeService, id: &str) -> ResourceId {
        let instance_id = ResourceId::new(id).unwrap();
        compute
            .launch(LaunchRequest {
                id: instance_id.clone(),
                account: account_id(),
                region: region_id(),
                principal: principal(),
                image_id: ResourceId::new("ami-1").unwrap(),
                capacity: Capacity::new(2, 4096),
                created_at: Timestamp::from_millis(1000),
            })
            .unwrap();
        instance_id
    }

    fn setup_messaging() -> MessagingService {
        let mut svc = MessagingService::new(allow_all_policy(), "core", "messaging");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.set_quota_limit("queues", 10);
        svc
    }

    fn create_queue(messaging: &mut MessagingService, id: &str) -> ResourceId {
        let queue_id = ResourceId::new(id).unwrap();
        messaging
            .create_queue(CreateQueueRequest {
                id: queue_id.clone(),
                account: account_id(),
                region: region_id(),
                principal: principal(),
                delivery: DeliverySemantics::AtLeastOnce,
                visibility_timeout_millis: 30_000,
                max_receives: 3,
                created_at: Timestamp::from_millis(1000),
            })
            .unwrap();
        queue_id
    }

    fn create_volume(storage: &mut StorageService, id: &str) -> ResourceId {
        let volume_id = ResourceId::new(id).unwrap();
        storage
            .create_volume(CreateVolumeRequest {
                id: volume_id.clone(),
                account: account_id(),
                region: region_id(),
                principal: principal(),
                size_bytes: 1024,
                redundancy: RedundancyScheme::replicated(3).unwrap(),
                created_at: Timestamp::from_millis(1000),
            })
            .unwrap();
        volume_id
    }

    fn setup_database() -> DatabaseService {
        let mut svc = DatabaseService::new(allow_all_policy(), "core", "database");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.set_quota_limit("databases", 10);
        svc
    }

    fn create_database(database: &mut DatabaseService, id: &str) -> ResourceId {
        let database_id = ResourceId::new(id).unwrap();
        database
            .create_database(CreateDatabaseRequest {
                id: database_id.clone(),
                account: account_id(),
                region: region_id(),
                principal: principal(),
                consistency: ConsistencyLevel::Eventual,
                retention: RetentionPolicy::new(86400000, 1).unwrap(),
                created_at: Timestamp::from_millis(1000),
            })
            .unwrap();
        database_id
    }

    #[test]
    fn attach_volume_rejects_an_unknown_instance() {
        let compute = setup_compute();
        let mut storage = setup_storage();
        let volume = create_volume(&mut storage, "vol-1");
        let err = attach_volume(
            &compute,
            &mut storage,
            &ResourceId::new("i-ghost").unwrap(),
            &volume,
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
        assert_eq!(
            storage.attachment_state(&volume),
            Some(AttachmentState::Detached),
            "a failed instance check must leave the volume untouched"
        );
    }

    #[test]
    fn attach_volume_rejects_a_terminated_instance() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        let volume = create_volume(&mut storage, "vol-1");
        compute
            .terminate(&instance, Timestamp::from_millis(2000))
            .unwrap();

        let err = attach_volume(&compute, &mut storage, &instance, &volume).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        assert_eq!(
            storage.attachment_state(&volume),
            Some(AttachmentState::Detached)
        );
    }

    #[test]
    fn attach_volume_succeeds_for_a_pending_instance() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        let volume = create_volume(&mut storage, "vol-1");

        let state = attach_volume(&compute, &mut storage, &instance, &volume).unwrap();
        assert_eq!(state, AttachmentState::Attached);
        assert_eq!(
            storage.attachment_state(&volume),
            Some(AttachmentState::Attached)
        );
    }

    #[test]
    fn attach_volume_succeeds_for_a_running_instance() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        compute
            .transition_runtime(&instance, RuntimeState::Running)
            .unwrap();
        let volume = create_volume(&mut storage, "vol-1");

        let state = attach_volume(&compute, &mut storage, &instance, &volume).unwrap();
        assert_eq!(state, AttachmentState::Attached);
    }

    #[test]
    fn attach_volume_of_an_unknown_volume_is_not_found() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        let err = attach_volume(
            &compute,
            &mut storage,
            &instance,
            &ResourceId::new("vol-ghost").unwrap(),
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn detach_volume_succeeds_even_after_its_instance_is_terminated() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        let volume = create_volume(&mut storage, "vol-1");
        attach_volume(&compute, &mut storage, &instance, &volume).unwrap();

        compute
            .terminate(&instance, Timestamp::from_millis(2000))
            .unwrap();

        let state = detach_volume(&mut storage, &volume).unwrap();
        assert_eq!(state, AttachmentState::Detached);
    }

    #[test]
    fn a_volume_can_be_reattached_after_detaching() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        let volume = create_volume(&mut storage, "vol-1");

        attach_volume(&compute, &mut storage, &instance, &volume).unwrap();
        detach_volume(&mut storage, &volume).unwrap();
        let state = attach_volume(&compute, &mut storage, &instance, &volume).unwrap();
        assert_eq!(state, AttachmentState::Attached);
    }

    #[test]
    fn attach_volume_of_an_already_attached_volume_is_rejected() {
        let mut compute = setup_compute();
        let mut storage = setup_storage();
        let instance = launch(&mut compute, "i-1");
        let volume = create_volume(&mut storage, "vol-1");
        attach_volume(&compute, &mut storage, &instance, &volume).unwrap();

        let err = attach_volume(&compute, &mut storage, &instance, &volume).unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
    }

    #[test]
    fn transition_runtime_and_notify_enqueues_and_transitions_together() {
        let mut compute = setup_compute();
        let mut messaging = setup_messaging();
        let instance = launch(&mut compute, "i-1");
        let queue = create_queue(&mut messaging, "q-1");

        let state = transition_runtime_and_notify(
            &mut compute,
            &mut messaging,
            &instance,
            RuntimeState::Running,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();

        assert_eq!(state, RuntimeState::Running);
        assert_eq!(
            compute.runtime_state(&instance),
            Some(RuntimeState::Running)
        );
        // The notification message is present and receivable.
        let outcome = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap();
        assert!(matches!(
            outcome,
            cloud_messaging::ReceiveOutcome::Delivered { .. }
        ));
    }

    #[test]
    fn transition_runtime_and_notify_rolls_back_the_message_on_an_invalid_transition() {
        let mut compute = setup_compute();
        let mut messaging = setup_messaging();
        let instance = launch(&mut compute, "i-1");
        let queue = create_queue(&mut messaging, "q-1");

        // Pending -> Stopped is not a valid runtime transition.
        let err = transition_runtime_and_notify(
            &mut compute,
            &mut messaging,
            &instance,
            RuntimeState::Stopped,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        // The rolled-back message must not still be sitting in the queue.
        let recv_err = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap_err();
        assert!(matches!(recv_err, CloudError::NotFound { .. }));
    }

    #[test]
    fn transition_runtime_and_notify_rejects_a_duplicate_message_id_and_touches_nothing() {
        let mut compute = setup_compute();
        let mut messaging = setup_messaging();
        let instance = launch(&mut compute, "i-1");
        let queue = create_queue(&mut messaging, "q-1");
        messaging
            .enqueue(
                &queue,
                ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(1500),
            )
            .unwrap();

        let err = transition_runtime_and_notify(
            &mut compute,
            &mut messaging,
            &instance,
            RuntimeState::Running,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::Conflict { .. }));
        // The runtime transition must never have been attempted.
        assert_eq!(
            compute.runtime_state(&instance),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn terminate_and_notify_enqueues_and_terminates_together() {
        let mut compute = setup_compute();
        let mut messaging = setup_messaging();
        let instance = launch(&mut compute, "i-1");
        let queue = create_queue(&mut messaging, "q-1");

        terminate_and_notify(
            &mut compute,
            &mut messaging,
            &instance,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();

        assert_eq!(
            compute.runtime_state(&instance),
            Some(RuntimeState::Terminated)
        );
        assert!(messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100)
            )
            .is_ok());
    }

    #[test]
    fn terminate_and_notify_rolls_back_the_message_for_an_unknown_instance() {
        let mut compute = setup_compute();
        let mut messaging = setup_messaging();
        let queue = create_queue(&mut messaging, "q-1");

        let err = terminate_and_notify(
            &mut compute,
            &mut messaging,
            &ResourceId::new("i-ghost").unwrap(),
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::NotFound { .. }));
        let recv_err = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap_err();
        assert!(matches!(recv_err, CloudError::NotFound { .. }));
    }

    #[test]
    fn apply_migration_and_notify_enqueues_and_migrates_together() {
        let mut database = setup_database();
        let mut messaging = setup_messaging();
        let db = create_database(&mut database, "db-1");
        let queue = create_queue(&mut messaging, "q-1");

        let version = apply_migration_and_notify(
            &mut database,
            &mut messaging,
            &db,
            1,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();

        assert_eq!(version, 1);
        assert_eq!(database.current_schema_version(&db), Some(1));
        let outcome = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap();
        assert!(matches!(
            outcome,
            cloud_messaging::ReceiveOutcome::Delivered { .. }
        ));
    }

    #[test]
    fn apply_migration_and_notify_rolls_back_on_invalid_migration() {
        let mut database = setup_database();
        let mut messaging = setup_messaging();
        let db = create_database(&mut database, "db-1");
        let queue = create_queue(&mut messaging, "q-1");

        // Version 2 requires version 1 first.
        let err = apply_migration_and_notify(
            &mut database,
            &mut messaging,
            &db,
            2,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        // The message must be rolled back.
        let recv_err = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap_err();
        assert!(matches!(recv_err, CloudError::NotFound { .. }));
        // Schema version unchanged.
        assert_eq!(database.current_schema_version(&db), Some(0));
    }

    #[test]
    fn delete_database_and_notify_enqueues_and_deletes_together() {
        let mut database = setup_database();
        let mut messaging = setup_messaging();
        let db = create_database(&mut database, "db-1");
        let queue = create_queue(&mut messaging, "q-1");

        delete_database_and_notify(
            &mut database,
            &mut messaging,
            &db,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();

        // The database exists but is now deleted.
        let deleted_db = database.get(&db).unwrap();
        assert_eq!(deleted_db.lifecycle(), Lifecycle::Deleted);
        let outcome = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap();
        assert!(matches!(
            outcome,
            cloud_messaging::ReceiveOutcome::Delivered { .. }
        ));
    }

    #[test]
    fn delete_database_and_notify_rolls_back_on_snapshots() {
        let mut database = setup_database();
        let mut messaging = setup_messaging();
        let db = create_database(&mut database, "db-1");
        let queue = create_queue(&mut messaging, "q-1");

        // Create a snapshot to block deletion.
        database
            .create_snapshot(
                &db,
                ResourceId::new("snap-1").unwrap(),
                Timestamp::from_millis(1500),
            )
            .unwrap();

        let err = delete_database_and_notify(
            &mut database,
            &mut messaging,
            &db,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::Conflict { .. }));
        // The message must be rolled back.
        let recv_err = messaging
            .receive(
                &queue,
                &ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap_err();
        assert!(matches!(recv_err, CloudError::NotFound { .. }));
        // Database still exists.
        assert!(database.get(&db).is_some());
    }

    #[test]
    fn delete_database_and_notify_rejects_duplicate_message_id_and_touches_nothing() {
        let mut database = setup_database();
        let mut messaging = setup_messaging();
        let db = create_database(&mut database, "db-1");
        let queue = create_queue(&mut messaging, "q-1");
        messaging
            .enqueue(
                &queue,
                ResourceId::new("evt-1").unwrap(),
                Timestamp::from_millis(1500),
            )
            .unwrap();

        let err = delete_database_and_notify(
            &mut database,
            &mut messaging,
            &db,
            &queue,
            ResourceId::new("evt-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap_err();

        assert!(matches!(err, CloudError::Conflict { .. }));
        // Database deletion must never have been attempted.
        assert!(database.get(&db).is_some());
    }
}
