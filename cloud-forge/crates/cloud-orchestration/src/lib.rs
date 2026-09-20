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
#![forbid(unsafe_code)]

use cloud_attachment::AttachmentState;
use cloud_compute::ComputeService;
use cloud_errors::CloudError;
use cloud_runtime::RuntimeState;
use cloud_storage::StorageService;
use cloud_types::ResourceId;

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

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_account::Account;
    use cloud_capacity::Capacity;
    use cloud_compute::LaunchRequest;
    use cloud_identity::Principal;
    use cloud_image::Architecture;
    use cloud_policy::{Effect, Policy, PrincipalMatcher, Statement};
    use cloud_redundancy::RedundancyScheme;
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
}
