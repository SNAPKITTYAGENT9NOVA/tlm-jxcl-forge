// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Per-availability-zone compute capacity: how many vCPUs and how
//! much memory (in MiB) an AZ actually has, and how much of that is
//! currently reserved.
//!
//! This is deliberately a different shape from [`cloud_quota`]'s
//! tracker, not a re-skin of it: `cloud-quota` treats an unconfigured
//! resource as *unlimited* -- an account cap that simply doesn't apply
//! until someone sets one. Capacity cannot default to unlimited: an
//! availability zone nobody ever told its own size has no real pool to
//! draw from, so [`CapacityTracker::try_reserve`] fails closed with
//! [`cloud_errors::CloudError::NotFound`] against an unregistered AZ,
//! never silently succeeding.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::AzId;
use std::collections::BTreeMap;

/// A vCPU + memory (MiB) quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capacity {
    pub vcpu: u64,
    pub memory_mib: u64,
}

impl Capacity {
    pub fn new(vcpu: u64, memory_mib: u64) -> Self {
        Capacity { vcpu, memory_mib }
    }

    fn saturating_add(self, other: Capacity) -> Capacity {
        Capacity {
            vcpu: self.vcpu.saturating_add(other.vcpu),
            memory_mib: self.memory_mib.saturating_add(other.memory_mib),
        }
    }

    fn saturating_sub(self, other: Capacity) -> Capacity {
        Capacity {
            vcpu: self.vcpu.saturating_sub(other.vcpu),
            memory_mib: self.memory_mib.saturating_sub(other.memory_mib),
        }
    }
}

/// Tracks each availability zone's total and currently-reserved
/// [`Capacity`].
#[derive(Debug, Clone, Default)]
pub struct CapacityTracker {
    total: BTreeMap<AzId, Capacity>,
    used: BTreeMap<AzId, Capacity>,
}

impl CapacityTracker {
    pub fn new() -> Self {
        CapacityTracker {
            total: BTreeMap::new(),
            used: BTreeMap::new(),
        }
    }

    /// Registers `az`'s total capacity. Rejects a second registration
    /// of the same AZ (its size is set once, at provisioning time, not
    /// silently resized by a later call) and a zero-sized total (an AZ
    /// with zero of either dimension has nothing to schedule onto).
    pub fn register_az(&mut self, az: AzId, total: Capacity) -> Result<(), CloudError> {
        if self.total.contains_key(&az) {
            return Err(CloudError::Conflict {
                what: "az capacity",
                id: az.to_string(),
            });
        }
        if total.vcpu == 0 || total.memory_mib == 0 {
            return Err(CloudError::InvalidFormat {
                what: "az capacity",
                value: format!("{}vcpu/{}mib", total.vcpu, total.memory_mib),
                reason: "vcpu and memory_mib must both be greater than zero".to_string(),
            });
        }
        self.total.insert(az, total);
        Ok(())
    }

    pub fn total(&self, az: &AzId) -> Option<Capacity> {
        self.total.get(az).copied()
    }

    pub fn used(&self, az: &AzId) -> Capacity {
        self.used.get(az).copied().unwrap_or_default()
    }

    /// `az`'s registered total minus its current usage, or `None` if
    /// `az` was never registered.
    pub fn available(&self, az: &AzId) -> Option<Capacity> {
        Some(self.total(az)?.saturating_sub(self.used(az)))
    }

    /// Attempts to reserve `amount` against `az`. Commits atomically:
    /// either both dimensions fit within what's left and both are
    /// reserved, or the reservation is rejected and usage is left
    /// completely unchanged.
    pub fn try_reserve(&mut self, az: &AzId, amount: Capacity) -> Result<(), CloudError> {
        let available = self.available(az).ok_or_else(|| CloudError::NotFound {
            what: "az capacity",
            id: az.to_string(),
        })?;
        if amount.vcpu > available.vcpu {
            return Err(CloudError::QuotaExceeded {
                resource: format!("{az} vcpu"),
                limit: available.vcpu,
                requested: amount.vcpu,
            });
        }
        if amount.memory_mib > available.memory_mib {
            return Err(CloudError::QuotaExceeded {
                resource: format!("{az} memory_mib"),
                limit: available.memory_mib,
                requested: amount.memory_mib,
            });
        }
        let updated = self.used(az).saturating_add(amount);
        self.used.insert(az.clone(), updated);
        Ok(())
    }

    /// Releases `amount` from `az`'s usage, saturating at zero. A
    /// no-op against an unregistered `az`, since a caller rolling back
    /// a reservation that never succeeded must not itself be able to
    /// fail.
    pub fn release(&mut self, az: &AzId, amount: Capacity) {
        if !self.total.contains_key(az) {
            return;
        }
        let updated = self.used(az).saturating_sub(amount);
        self.used.insert(az.clone(), updated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn az(s: &str) -> AzId {
        AzId::new(s).unwrap()
    }

    #[test]
    fn register_creates_capacity_with_zero_usage() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(16, 65536))
            .unwrap();
        assert_eq!(t.total(&az("us-west-1a")), Some(Capacity::new(16, 65536)));
        assert_eq!(t.used(&az("us-west-1a")), Capacity::default());
        assert_eq!(
            t.available(&az("us-west-1a")),
            Some(Capacity::new(16, 65536))
        );
    }

    #[test]
    fn registering_the_same_az_twice_is_a_conflict() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(16, 65536))
            .unwrap();
        assert!(matches!(
            t.register_az(az("us-west-1a"), Capacity::new(8, 32768)),
            Err(CloudError::Conflict { .. })
        ));
    }

    #[test]
    fn registering_zero_vcpu_or_zero_memory_is_rejected() {
        let mut t = CapacityTracker::new();
        assert!(t
            .register_az(az("us-west-1a"), Capacity::new(0, 65536))
            .is_err());
        assert!(t
            .register_az(az("us-west-1b"), Capacity::new(16, 0))
            .is_err());
    }

    #[test]
    fn reserving_within_capacity_succeeds_and_updates_used() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(16, 65536))
            .unwrap();
        t.try_reserve(&az("us-west-1a"), Capacity::new(4, 8192))
            .unwrap();
        assert_eq!(t.used(&az("us-west-1a")), Capacity::new(4, 8192));
        assert_eq!(
            t.available(&az("us-west-1a")),
            Some(Capacity::new(12, 57344))
        );
    }

    #[test]
    fn reserving_exactly_the_available_amount_succeeds() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(4, 4096))
            .unwrap();
        t.try_reserve(&az("us-west-1a"), Capacity::new(4, 4096))
            .unwrap();
        assert_eq!(t.available(&az("us-west-1a")), Some(Capacity::default()));
    }

    #[test]
    fn reserving_past_vcpu_capacity_fails_and_leaves_usage_unchanged() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(4, 65536))
            .unwrap();
        let err = t
            .try_reserve(&az("us-west-1a"), Capacity::new(5, 1024))
            .unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(t.used(&az("us-west-1a")), Capacity::default());
    }

    #[test]
    fn reserving_past_memory_capacity_fails_and_leaves_usage_unchanged() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(16, 4096))
            .unwrap();
        let err = t
            .try_reserve(&az("us-west-1a"), Capacity::new(1, 8192))
            .unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(t.used(&az("us-west-1a")), Capacity::default());
    }

    #[test]
    fn reserving_against_an_unregistered_az_is_not_found() {
        let mut t = CapacityTracker::new();
        let err = t
            .try_reserve(&az("us-west-1a"), Capacity::new(1, 1))
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn release_decreases_usage() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(16, 65536))
            .unwrap();
        t.try_reserve(&az("us-west-1a"), Capacity::new(8, 16384))
            .unwrap();
        t.release(&az("us-west-1a"), Capacity::new(3, 4096));
        assert_eq!(t.used(&az("us-west-1a")), Capacity::new(5, 12288));
    }

    #[test]
    fn release_saturates_at_zero_rather_than_underflowing() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(16, 65536))
            .unwrap();
        t.try_reserve(&az("us-west-1a"), Capacity::new(2, 2048))
            .unwrap();
        t.release(&az("us-west-1a"), Capacity::new(100, 999_999));
        assert_eq!(t.used(&az("us-west-1a")), Capacity::default());
    }

    #[test]
    fn release_against_an_unregistered_az_is_a_noop() {
        let mut t = CapacityTracker::new();
        t.release(&az("us-west-1a"), Capacity::new(1, 1));
        assert_eq!(t.total(&az("us-west-1a")), None);
        assert_eq!(t.used(&az("us-west-1a")), Capacity::default());
    }

    #[test]
    fn released_capacity_can_be_reserved_again() {
        let mut t = CapacityTracker::new();
        t.register_az(az("us-west-1a"), Capacity::new(4, 4096))
            .unwrap();
        t.try_reserve(&az("us-west-1a"), Capacity::new(4, 4096))
            .unwrap();
        assert!(t
            .try_reserve(&az("us-west-1a"), Capacity::new(1, 1))
            .is_err());
        t.release(&az("us-west-1a"), Capacity::new(1, 1024));
        t.try_reserve(&az("us-west-1a"), Capacity::new(1, 1024))
            .unwrap();
        assert_eq!(t.used(&az("us-west-1a")), Capacity::new(4, 4096));
    }
}
