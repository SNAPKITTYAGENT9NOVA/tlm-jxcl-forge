//! `Resource<T>`: the generic wrapper every `cloud-forge` resource is
//! built from, so a future `Instance`/`Bucket`/etc. is `Resource<InstanceSpec>`
//! rather than a hand-rolled struct that has to remember to include
//! `tags`, `lifecycle`, and a version counter correctly on its own.
//!
//! ## The version invariant
//!
//! `version` only ever increases, and only as part of the same
//! operation that actually changed something. Every mutating method
//! here is atomic: on failure (an invalid lifecycle transition, an
//! invalid tag), the resource is left completely unchanged -- no
//! partial update, no version bump for a mutation that didn't happen.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_lifecycle::Lifecycle;
use cloud_tags::Tags;
use cloud_types::{AccountId, RegionId, ResourceId, ResourceType, Timestamp};

/// A cloud resource wrapping a service-specific payload `T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource<T> {
    id: ResourceId,
    resource_type: ResourceType,
    /// `None` for a global resource (e.g. an account itself).
    region: Option<RegionId>,
    account: AccountId,
    lifecycle: Lifecycle,
    version: u64,
    tags: Tags,
    created_at: Timestamp,
    updated_at: Timestamp,
    payload: T,
}

impl<T> Resource<T> {
    /// Creates a new resource in [`Lifecycle::Creating`] at version 1,
    /// with no tags.
    pub fn new(
        id: ResourceId,
        resource_type: ResourceType,
        region: Option<RegionId>,
        account: AccountId,
        created_at: Timestamp,
        payload: T,
    ) -> Self {
        Resource {
            id,
            resource_type,
            region,
            account,
            lifecycle: Lifecycle::Creating,
            version: 1,
            tags: Tags::new(),
            created_at,
            updated_at: created_at,
            payload,
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
    }

    pub fn resource_type(&self) -> &ResourceType {
        &self.resource_type
    }

    pub fn region(&self) -> Option<&RegionId> {
        self.region.as_ref()
    }

    pub fn account(&self) -> &AccountId {
        &self.account
    }

    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn tags(&self) -> &Tags {
        &self.tags
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    fn bump(&mut self, now: Timestamp) {
        self.version += 1;
        self.updated_at = now;
    }

    /// Moves this resource's lifecycle forward, bumping `version` and
    /// `updated_at` only if the transition is actually allowed.
    pub fn transition_lifecycle(
        &mut self,
        to: Lifecycle,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        let next = self.lifecycle.transition_to(to)?;
        self.lifecycle = next;
        self.bump(now);
        Ok(())
    }

    /// Sets a tag, bumping `version`/`updated_at` only if the tag is
    /// valid. Returns the previous value of `key`, if any.
    pub fn set_tag(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        now: Timestamp,
    ) -> Result<Option<String>, CloudError> {
        let previous = self.tags.insert(key, value)?;
        self.bump(now);
        Ok(previous)
    }

    /// Removes a tag, bumping `version`/`updated_at` only if a tag was
    /// actually present to remove.
    pub fn remove_tag(&mut self, key: &str, now: Timestamp) -> Option<String> {
        let removed = self.tags.remove(key);
        if removed.is_some() {
            self.bump(now);
        }
        removed
    }

    /// Replaces the payload unconditionally, bumping `version`/`updated_at`.
    /// Payload updates are not gated by lifecycle state in this phase
    /// -- see this crate's doc comment.
    pub fn update_payload(&mut self, payload: T, now: Timestamp) {
        self.payload = payload;
        self.bump(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> AccountId {
        AccountId::new("000000000001").unwrap()
    }

    fn new_resource() -> Resource<u32> {
        Resource::new(
            ResourceId::new("i-1").unwrap(),
            ResourceType::new("compute-instance").unwrap(),
            Some(RegionId::new("us-west-1").unwrap()),
            account(),
            Timestamp::from_millis(1000),
            42,
        )
    }

    #[test]
    fn new_resource_starts_at_version_one_in_creating() {
        let r = new_resource();
        assert_eq!(r.version(), 1);
        assert_eq!(r.lifecycle(), Lifecycle::Creating);
        assert_eq!(r.created_at(), r.updated_at());
        assert_eq!(*r.payload(), 42);
    }

    #[test]
    fn valid_transition_bumps_version_and_updated_at() {
        let mut r = new_resource();
        r.transition_lifecycle(Lifecycle::Active, Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(r.lifecycle(), Lifecycle::Active);
        assert_eq!(r.version(), 2);
        assert_eq!(r.updated_at(), Timestamp::from_millis(2000));
        // created_at must never change.
        assert_eq!(r.created_at(), Timestamp::from_millis(1000));
    }

    #[test]
    fn invalid_transition_leaves_the_resource_completely_unchanged() {
        let mut r = new_resource();
        let before = r.clone();
        let err = r
            .transition_lifecycle(Lifecycle::Deleted, Timestamp::from_millis(2000))
            .unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert_eq!(r, before, "a rejected transition must not mutate anything");
    }

    #[test]
    fn set_tag_bumps_version_only_on_success() {
        let mut r = new_resource();
        r.set_tag("env", "prod", Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(r.version(), 2);
        assert_eq!(r.tags().get("env"), Some("prod"));

        let before = r.clone();
        let err = r
            .set_tag("jxcl:reserved", "x", Timestamp::from_millis(3000))
            .unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
        assert_eq!(r, before, "a rejected tag insert must not mutate anything");
    }

    #[test]
    fn remove_tag_only_bumps_version_when_something_was_removed() {
        let mut r = new_resource();
        r.set_tag("env", "prod", Timestamp::from_millis(2000))
            .unwrap();
        assert_eq!(r.version(), 2);

        assert!(r
            .remove_tag("does-not-exist", Timestamp::from_millis(3000))
            .is_none());
        assert_eq!(
            r.version(),
            2,
            "removing a nonexistent tag must not bump version"
        );

        assert_eq!(
            r.remove_tag("env", Timestamp::from_millis(4000)),
            Some("prod".to_string())
        );
        assert_eq!(r.version(), 3);
    }

    #[test]
    fn update_payload_bumps_version_and_replaces_the_value() {
        let mut r = new_resource();
        r.update_payload(99, Timestamp::from_millis(2000));
        assert_eq!(*r.payload(), 99);
        assert_eq!(r.version(), 2);
    }

    #[test]
    fn version_increases_monotonically_across_a_sequence_of_operations() {
        let mut r = new_resource();
        let mut last_version = r.version();
        let ops: [fn(&mut Resource<u32>); 3] = [
            |r| {
                r.set_tag("a", "1", Timestamp::from_millis(2000)).unwrap();
            },
            |r| {
                r.transition_lifecycle(Lifecycle::Active, Timestamp::from_millis(3000))
                    .unwrap();
            },
            |r| r.update_payload(7, Timestamp::from_millis(4000)),
        ];
        for op in ops {
            op(&mut r);
            assert!(r.version() > last_version);
            last_version = r.version();
        }
    }
}
