// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! An [`Arn`] ↔ [`ResourceId`] registry: the one place "what does this
//! fully-qualified name actually point to?" gets answered.
//!
//! `Resource<T>` (in `cloud-resource`) knows its own bare `ResourceId`,
//! but nothing before this crate ever records a resource's *canonical
//! name* -- the `Arn` a caller outside the system that created it
//! would actually use to refer to it. This registry is deliberately
//! narrow: it owns the name-to-resource mapping and nothing else (not
//! the resource itself, not its lifecycle) -- `cloud-control-plane` is
//! what actually calls it, right after provisioning a resource.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::{Arn, ResourceId};
use std::collections::BTreeMap;

/// A registry mapping [`Arn`]s to the [`ResourceId`]s they name.
///
/// `Arn` itself has no total order (its fields are a mix of optional
/// and required parts with no natural ranking), so this registry
/// indexes by each `Arn`'s canonical string form (its `Display`
/// output) rather than by `Arn` directly -- reconstructing the `Arn`
/// on lookup via its own `FromStr` impl is a cheap, always-correct
/// round trip, since `Arn`'s `Display`/`FromStr` pair is exact.
#[derive(Debug, Clone, Default)]
pub struct ResourceRegistry {
    by_arn: BTreeMap<String, ResourceId>,
    by_resource: BTreeMap<ResourceId, String>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        ResourceRegistry {
            by_arn: BTreeMap::new(),
            by_resource: BTreeMap::new(),
        }
    }

    /// Registers `arn` as `id`'s canonical name. Rejects a second
    /// registration of an already-used `arn` (even for the same
    /// `id`), and rejects registering `id` under a second `arn` (a
    /// resource's canonical name is chosen once, at creation, and
    /// does not change).
    pub fn register(&mut self, arn: &Arn, id: ResourceId) -> Result<(), CloudError> {
        let key = arn.to_string();
        if self.by_arn.contains_key(&key) {
            return Err(CloudError::Conflict {
                what: "arn",
                id: key,
            });
        }
        if self.by_resource.contains_key(&id) {
            return Err(CloudError::Conflict {
                what: "resource",
                id: id.to_string(),
            });
        }
        self.by_arn.insert(key.clone(), id.clone());
        self.by_resource.insert(id, key);
        Ok(())
    }

    /// The `ResourceId` `arn` names, if any.
    pub fn resolve(&self, arn: &Arn) -> Option<&ResourceId> {
        self.by_arn.get(&arn.to_string())
    }

    /// `id`'s canonical `Arn`, if it has been registered.
    pub fn arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.by_resource
            .get(id)
            .and_then(|arn_string| arn_string.parse().ok())
    }

    pub fn contains_arn(&self, arn: &Arn) -> bool {
        self.by_arn.contains_key(&arn.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_types::{AccountId, RegionId};

    fn arn(resource: &str) -> Arn {
        Arn::new(
            "core",
            "compute",
            Some(RegionId::new("us-west-1").unwrap()),
            Some(AccountId::new("000000000001").unwrap()),
            ResourceId::new(resource).unwrap(),
        )
        .unwrap()
    }

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    #[test]
    fn register_then_resolve_round_trips() {
        let mut reg = ResourceRegistry::new();
        let a = arn("i-1");
        reg.register(&a, id("i-1")).unwrap();
        assert_eq!(reg.resolve(&a), Some(&id("i-1")));
    }

    #[test]
    fn arn_of_reconstructs_the_original_arn() {
        let mut reg = ResourceRegistry::new();
        let a = arn("i-1");
        reg.register(&a, id("i-1")).unwrap();
        assert_eq!(reg.arn_of(&id("i-1")), Some(a));
    }

    #[test]
    fn duplicate_arn_registration_is_rejected_even_for_a_different_resource() {
        let mut reg = ResourceRegistry::new();
        let a = arn("i-1");
        reg.register(&a, id("i-1")).unwrap();
        let err = reg.register(&a, id("i-2")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        // The original registration must be untouched.
        assert_eq!(reg.resolve(&a), Some(&id("i-1")));
    }

    #[test]
    fn a_resource_can_only_be_registered_under_one_arn() {
        let mut reg = ResourceRegistry::new();
        reg.register(&arn("i-1"), id("i-1")).unwrap();
        let err = reg.register(&arn("i-2"), id("i-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn resolving_an_unregistered_arn_returns_none() {
        let reg = ResourceRegistry::new();
        assert_eq!(reg.resolve(&arn("i-1")), None);
    }

    #[test]
    fn arn_of_an_unregistered_resource_returns_none() {
        let reg = ResourceRegistry::new();
        assert_eq!(reg.arn_of(&id("i-1")), None);
    }

    #[test]
    fn contains_arn_reflects_registration_state() {
        let mut reg = ResourceRegistry::new();
        let a = arn("i-1");
        assert!(!reg.contains_arn(&a));
        reg.register(&a, id("i-1")).unwrap();
        assert!(reg.contains_arn(&a));
    }
}
