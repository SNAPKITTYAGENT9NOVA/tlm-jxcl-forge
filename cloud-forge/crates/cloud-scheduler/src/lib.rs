// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Least-loaded-AZ placement.
//!
//! This crate does not track load itself -- it is handed a `usage`
//! map by the caller (typically `cloud-provisioner` or
//! `cloud-control-plane`, which own the actual resource store) and
//! both reads and updates it. Keeping load ownership external means
//! this crate has no persistence, no state, and nothing to get out of
//! sync with reality.
#![forbid(unsafe_code)]

use cloud_capacity::{Capacity, CapacityTracker};
use cloud_errors::CloudError;
use cloud_region::RegionRegistry;
use cloud_types::{AzId, RegionId};
use std::collections::BTreeMap;

/// Picks the least-loaded availability zone in `region`, using
/// `usage` as each AZ's current load (an AZ absent from `usage` is
/// treated as having zero load). Ties are broken by `AzId` ordering,
/// so the result is deterministic for a given `usage` map. On success,
/// the winning AZ's entry in `usage` is incremented by one.
pub fn place_least_loaded(
    registry: &RegionRegistry,
    region: &RegionId,
    usage: &mut BTreeMap<AzId, u64>,
) -> Result<AzId, CloudError> {
    let info = registry.get(region).ok_or_else(|| CloudError::NotFound {
        what: "region",
        id: region.to_string(),
    })?;

    let winner = info
        .azs()
        .iter()
        .min_by_key(|az| (usage.get(*az).copied().unwrap_or(0), (*az).clone()))
        .ok_or_else(|| CloudError::InvalidFormat {
            what: "region",
            value: region.to_string(),
            reason: "has no availability zones registered".to_string(),
        })?
        .clone();

    *usage.entry(winner.clone()).or_insert(0) += 1;
    Ok(winner)
}

/// Like [`place_least_loaded`], but only considers availability zones
/// in `region` with enough spare [`Capacity`] for `required`, and
/// reserves that capacity in `capacity` on the winning AZ alongside
/// incrementing its load in `usage`.
///
/// Fails with [`CloudError::QuotaExceeded`] if no AZ in `region`
/// currently has enough room, without touching `usage` or `capacity`
/// for any candidate -- the same all-or-nothing discipline
/// [`cloud_capacity::CapacityTracker::try_reserve`] itself keeps.
pub fn place_least_loaded_with_capacity(
    registry: &RegionRegistry,
    capacity: &mut CapacityTracker,
    region: &RegionId,
    usage: &mut BTreeMap<AzId, u64>,
    required: Capacity,
) -> Result<AzId, CloudError> {
    let info = registry.get(region).ok_or_else(|| CloudError::NotFound {
        what: "region",
        id: region.to_string(),
    })?;

    let winner = info
        .azs()
        .iter()
        .filter(|az| {
            capacity.available(az).is_some_and(|avail| {
                avail.vcpu >= required.vcpu && avail.memory_mib >= required.memory_mib
            })
        })
        .min_by_key(|az| (usage.get(*az).copied().unwrap_or(0), (*az).clone()))
        .cloned()
        .ok_or_else(|| CloudError::QuotaExceeded {
            resource: format!("{region} capacity"),
            limit: 0,
            requested: required.vcpu.max(required.memory_mib),
        })?;

    capacity.try_reserve(&winner, required)?;
    *usage.entry(winner.clone()).or_insert(0) += 1;
    Ok(winner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with(region: &str, az_count: u32) -> RegionRegistry {
        let mut reg = RegionRegistry::new();
        reg.register(RegionId::new(region).unwrap(), az_count)
            .unwrap();
        reg
    }

    #[test]
    fn places_in_the_only_az_when_there_is_just_one() {
        let reg = registry_with("us-west-1", 1);
        let mut usage = BTreeMap::new();
        let az =
            place_least_loaded(&reg, &RegionId::new("us-west-1").unwrap(), &mut usage).unwrap();
        assert_eq!(az, AzId::new("us-west-1a").unwrap());
    }

    #[test]
    fn picks_the_az_with_lowest_usage() {
        let reg = registry_with("us-west-1", 3);
        let mut usage = BTreeMap::new();
        usage.insert(AzId::new("us-west-1a").unwrap(), 5);
        usage.insert(AzId::new("us-west-1b").unwrap(), 1);
        usage.insert(AzId::new("us-west-1c").unwrap(), 3);

        let az =
            place_least_loaded(&reg, &RegionId::new("us-west-1").unwrap(), &mut usage).unwrap();
        assert_eq!(az, AzId::new("us-west-1b").unwrap());
    }

    #[test]
    fn ties_break_deterministically_by_az_id_order() {
        let reg = registry_with("us-west-1", 3);
        // All AZs absent from `usage` -- all tied at zero load.
        let mut usage = BTreeMap::new();
        let az =
            place_least_loaded(&reg, &RegionId::new("us-west-1").unwrap(), &mut usage).unwrap();
        assert_eq!(az, AzId::new("us-west-1a").unwrap());
    }

    #[test]
    fn placement_increments_the_winners_usage_count() {
        let reg = registry_with("us-west-1", 1);
        let mut usage = BTreeMap::new();
        place_least_loaded(&reg, &RegionId::new("us-west-1").unwrap(), &mut usage).unwrap();
        assert_eq!(usage[&AzId::new("us-west-1a").unwrap()], 1);
        place_least_loaded(&reg, &RegionId::new("us-west-1").unwrap(), &mut usage).unwrap();
        assert_eq!(usage[&AzId::new("us-west-1a").unwrap()], 2);
    }

    #[test]
    fn repeated_placement_spreads_load_evenly_across_azs() {
        let reg = registry_with("us-west-1", 3);
        let mut usage = BTreeMap::new();
        let region = RegionId::new("us-west-1").unwrap();
        let mut picks = Vec::new();
        for _ in 0..6 {
            picks.push(place_least_loaded(&reg, &region, &mut usage).unwrap());
        }
        // Every AZ should have received exactly 2 of the 6 placements.
        for az in ["us-west-1a", "us-west-1b", "us-west-1c"] {
            let count = picks.iter().filter(|p| p.as_str() == az).count();
            assert_eq!(count, 2, "{az} should have received exactly 2 placements");
        }
    }

    #[test]
    fn errors_on_an_unregistered_region() {
        let reg = RegionRegistry::new();
        let mut usage = BTreeMap::new();
        let err =
            place_least_loaded(&reg, &RegionId::new("us-west-1").unwrap(), &mut usage).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    fn az(s: &str) -> AzId {
        AzId::new(s).unwrap()
    }

    #[test]
    fn capacity_aware_placement_picks_the_least_loaded_az_with_room() {
        let reg = registry_with("us-west-1", 2);
        let mut cap = CapacityTracker::new();
        cap.register_az(az("us-west-1a"), Capacity::new(16, 65536))
            .unwrap();
        cap.register_az(az("us-west-1b"), Capacity::new(16, 65536))
            .unwrap();
        let mut usage = BTreeMap::new();
        usage.insert(az("us-west-1a"), 5);

        let winner = place_least_loaded_with_capacity(
            &reg,
            &mut cap,
            &RegionId::new("us-west-1").unwrap(),
            &mut usage,
            Capacity::new(2, 4096),
        )
        .unwrap();
        assert_eq!(winner, az("us-west-1b"));
        assert_eq!(cap.used(&az("us-west-1b")), Capacity::new(2, 4096));
    }

    #[test]
    fn capacity_aware_placement_skips_an_az_without_enough_vcpu() {
        let reg = registry_with("us-west-1", 2);
        let mut cap = CapacityTracker::new();
        // The least-loaded AZ has room, but not enough vCPU.
        cap.register_az(az("us-west-1a"), Capacity::new(1, 65536))
            .unwrap();
        cap.register_az(az("us-west-1b"), Capacity::new(16, 65536))
            .unwrap();
        let mut usage = BTreeMap::new();

        let winner = place_least_loaded_with_capacity(
            &reg,
            &mut cap,
            &RegionId::new("us-west-1").unwrap(),
            &mut usage,
            Capacity::new(4, 1024),
        )
        .unwrap();
        assert_eq!(winner, az("us-west-1b"));
    }

    #[test]
    fn capacity_aware_placement_skips_an_az_without_enough_memory() {
        let reg = registry_with("us-west-1", 2);
        let mut cap = CapacityTracker::new();
        cap.register_az(az("us-west-1a"), Capacity::new(16, 1024))
            .unwrap();
        cap.register_az(az("us-west-1b"), Capacity::new(16, 65536))
            .unwrap();
        let mut usage = BTreeMap::new();

        let winner = place_least_loaded_with_capacity(
            &reg,
            &mut cap,
            &RegionId::new("us-west-1").unwrap(),
            &mut usage,
            Capacity::new(4, 8192),
        )
        .unwrap();
        assert_eq!(winner, az("us-west-1b"));
    }

    #[test]
    fn capacity_aware_placement_fails_when_no_az_has_room_and_touches_nothing() {
        let reg = registry_with("us-west-1", 2);
        let mut cap = CapacityTracker::new();
        cap.register_az(az("us-west-1a"), Capacity::new(2, 2048))
            .unwrap();
        cap.register_az(az("us-west-1b"), Capacity::new(2, 2048))
            .unwrap();
        let mut usage = BTreeMap::new();

        let err = place_least_loaded_with_capacity(
            &reg,
            &mut cap,
            &RegionId::new("us-west-1").unwrap(),
            &mut usage,
            Capacity::new(4, 4096),
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert!(usage.is_empty());
        assert_eq!(cap.used(&az("us-west-1a")), Capacity::default());
        assert_eq!(cap.used(&az("us-west-1b")), Capacity::default());
    }

    #[test]
    fn capacity_aware_placement_errors_on_an_unregistered_region() {
        let reg = RegionRegistry::new();
        let mut cap = CapacityTracker::new();
        let mut usage = BTreeMap::new();
        let err = place_least_loaded_with_capacity(
            &reg,
            &mut cap,
            &RegionId::new("us-west-1").unwrap(),
            &mut usage,
            Capacity::new(1, 1),
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }
}
