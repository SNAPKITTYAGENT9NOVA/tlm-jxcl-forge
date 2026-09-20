// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A quota/usage tracker.
//!
//! Implements the roadmap's **CLOUD-I009: quota violations cannot
//! commit** directly: [`QuotaTracker::try_reserve`] either commits the
//! full requested amount or leaves usage completely unchanged and
//! returns [`cloud_errors::CloudError::QuotaExceeded`] -- there is no
//! partial reservation.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use std::collections::BTreeMap;

/// Tracks a limit and current usage per named resource category (e.g.
/// `"instances"`, `"volumes"`). A category with no configured limit is
/// unlimited -- quotas in this phase are opt-in caps on top of an
/// otherwise-unrestricted default, not a default-deny allowlist (that
/// is `cloud-policy`'s job).
#[derive(Debug, Clone, Default)]
pub struct QuotaTracker {
    limits: BTreeMap<String, u64>,
    usage: BTreeMap<String, u64>,
}

impl QuotaTracker {
    pub fn new() -> Self {
        QuotaTracker {
            limits: BTreeMap::new(),
            usage: BTreeMap::new(),
        }
    }

    pub fn set_limit(&mut self, resource: impl Into<String>, limit: u64) {
        self.limits.insert(resource.into(), limit);
    }

    pub fn limit(&self, resource: &str) -> Option<u64> {
        self.limits.get(resource).copied()
    }

    pub fn usage(&self, resource: &str) -> u64 {
        self.usage.get(resource).copied().unwrap_or(0)
    }

    /// Attempts to increase `resource`'s usage by `amount`. Commits
    /// atomically: either the whole amount is reserved, or (if a
    /// configured limit would be exceeded) usage is left completely
    /// unchanged and [`CloudError::QuotaExceeded`] is returned.
    pub fn try_reserve(&mut self, resource: &str, amount: u64) -> Result<(), CloudError> {
        let current = self.usage(resource);
        let requested_total = current.saturating_add(amount);
        if let Some(limit) = self.limit(resource) {
            if requested_total > limit {
                return Err(CloudError::QuotaExceeded {
                    resource: resource.to_string(),
                    limit,
                    requested: requested_total,
                });
            }
        }
        self.usage.insert(resource.to_string(), requested_total);
        Ok(())
    }

    /// Releases `amount` of `resource`'s usage, saturating at zero
    /// rather than underflowing if `amount` exceeds current usage.
    pub fn release(&mut self, resource: &str, amount: u64) {
        let current = self.usage(resource);
        self.usage
            .insert(resource.to_string(), current.saturating_sub(amount));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserving_within_the_limit_succeeds_and_updates_usage() {
        let mut q = QuotaTracker::new();
        q.set_limit("instances", 5);
        q.try_reserve("instances", 3).unwrap();
        assert_eq!(q.usage("instances"), 3);
    }

    #[test]
    fn reserving_exactly_up_to_the_limit_succeeds() {
        let mut q = QuotaTracker::new();
        q.set_limit("instances", 5);
        q.try_reserve("instances", 5).unwrap();
        assert_eq!(q.usage("instances"), 5);
    }

    #[test]
    fn reserving_past_the_limit_fails_and_leaves_usage_unchanged() {
        let mut q = QuotaTracker::new();
        q.set_limit("instances", 5);
        q.try_reserve("instances", 3).unwrap();

        let err = q.try_reserve("instances", 3).unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(
            q.usage("instances"),
            3,
            "a rejected reservation must not commit any part of itself"
        );
    }

    #[test]
    fn an_unconfigured_resource_is_unlimited() {
        let mut q = QuotaTracker::new();
        q.try_reserve("anything", u64::MAX / 2).unwrap();
        assert_eq!(q.usage("anything"), u64::MAX / 2);
    }

    #[test]
    fn release_decreases_usage() {
        let mut q = QuotaTracker::new();
        q.set_limit("instances", 10);
        q.try_reserve("instances", 5).unwrap();
        q.release("instances", 2);
        assert_eq!(q.usage("instances"), 3);
    }

    #[test]
    fn release_saturates_at_zero_rather_than_underflowing() {
        let mut q = QuotaTracker::new();
        q.try_reserve("instances", 2).unwrap();
        q.release("instances", 100);
        assert_eq!(q.usage("instances"), 0);
    }

    #[test]
    fn released_capacity_can_be_reserved_again() {
        let mut q = QuotaTracker::new();
        q.set_limit("instances", 5);
        q.try_reserve("instances", 5).unwrap();
        assert!(q.try_reserve("instances", 1).is_err());
        q.release("instances", 1);
        assert!(q.try_reserve("instances", 1).is_ok());
        assert_eq!(q.usage("instances"), 5);
    }
}
