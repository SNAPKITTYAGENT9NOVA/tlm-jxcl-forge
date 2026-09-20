// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A registry of named control-plane services and their health.
//!
//! This is deliberately small: it tracks *that* a service exists and
//! *what its last-reported health is*, not how health is determined
//! (a heartbeat protocol, a liveness probe, etc. -- all later-phase
//! concerns). Service names reuse `cloud_types::ResourceType`'s
//! validated kebab-case shape rather than a bare `String`, since a
//! service name is exactly that kind of identifier.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::ResourceType;
use std::collections::BTreeMap;

/// A service's last-reported health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Unhealthy,
    /// No health report has been recorded yet -- the default for a
    /// freshly registered service.
    Unknown,
}

/// A registry of services and their health.
#[derive(Debug, Clone, Default)]
pub struct ServiceRegistry {
    services: BTreeMap<ResourceType, Health>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: BTreeMap::new(),
        }
    }

    /// Registers `name`, starting at [`Health::Unknown`]. Rejects a
    /// name already registered.
    pub fn register(&mut self, name: ResourceType) -> Result<(), CloudError> {
        if self.services.contains_key(&name) {
            return Err(CloudError::Conflict {
                what: "service",
                id: name.to_string(),
            });
        }
        self.services.insert(name, Health::Unknown);
        Ok(())
    }

    /// Updates a registered service's health. Fails if `name` was
    /// never registered.
    pub fn set_health(&mut self, name: &ResourceType, health: Health) -> Result<(), CloudError> {
        let slot = self
            .services
            .get_mut(name)
            .ok_or_else(|| CloudError::NotFound {
                what: "service",
                id: name.to_string(),
            })?;
        *slot = health;
        Ok(())
    }

    pub fn health(&self, name: &ResourceType) -> Option<Health> {
        self.services.get(name).copied()
    }

    pub fn is_registered(&self, name: &ResourceType) -> bool {
        self.services.contains_key(name)
    }

    /// All registered services and their health, in deterministic
    /// (name-sorted) order.
    pub fn services(&self) -> impl Iterator<Item = (&ResourceType, Health)> {
        self.services.iter().map(|(name, health)| (name, *health))
    }

    /// Registered services currently reporting [`Health::Healthy`],
    /// in deterministic (name-sorted) order.
    pub fn healthy_services(&self) -> impl Iterator<Item = &ResourceType> {
        self.services
            .iter()
            .filter(|(_, health)| **health == Health::Healthy)
            .map(|(name, _)| name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> ResourceType {
        ResourceType::new(s).unwrap()
    }

    #[test]
    fn registering_starts_at_unknown_health() {
        let mut reg = ServiceRegistry::new();
        reg.register(name("scheduler")).unwrap();
        assert_eq!(reg.health(&name("scheduler")), Some(Health::Unknown));
    }

    #[test]
    fn registering_the_same_name_twice_is_a_conflict() {
        let mut reg = ServiceRegistry::new();
        reg.register(name("scheduler")).unwrap();
        assert!(matches!(
            reg.register(name("scheduler")),
            Err(CloudError::Conflict { .. })
        ));
    }

    #[test]
    fn set_health_updates_a_registered_service() {
        let mut reg = ServiceRegistry::new();
        reg.register(name("scheduler")).unwrap();
        reg.set_health(&name("scheduler"), Health::Healthy).unwrap();
        assert_eq!(reg.health(&name("scheduler")), Some(Health::Healthy));
    }

    #[test]
    fn set_health_on_an_unregistered_service_fails() {
        let mut reg = ServiceRegistry::new();
        let err = reg
            .set_health(&name("scheduler"), Health::Healthy)
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn health_of_an_unregistered_service_is_none() {
        let reg = ServiceRegistry::new();
        assert_eq!(reg.health(&name("scheduler")), None);
        assert!(!reg.is_registered(&name("scheduler")));
    }

    #[test]
    fn services_are_iterated_in_sorted_order() {
        let mut reg = ServiceRegistry::new();
        reg.register(name("scheduler")).unwrap();
        reg.register(name("provisioner")).unwrap();
        reg.register(name("reconciler")).unwrap();
        let names: Vec<String> = reg.services().map(|(n, _)| n.to_string()).collect();
        assert_eq!(names, vec!["provisioner", "reconciler", "scheduler"]);
    }

    #[test]
    fn healthy_services_filters_correctly() {
        let mut reg = ServiceRegistry::new();
        reg.register(name("scheduler")).unwrap();
        reg.register(name("provisioner")).unwrap();
        reg.set_health(&name("scheduler"), Health::Healthy).unwrap();
        reg.set_health(&name("provisioner"), Health::Unhealthy)
            .unwrap();
        let healthy: Vec<String> = reg.healthy_services().map(ToString::to_string).collect();
        assert_eq!(healthy, vec!["scheduler"]);
    }
}
