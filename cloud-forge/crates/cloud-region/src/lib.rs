//! A registry of which availability zones exist under which region.
//!
//! [`cloud_types::AzId`] validates an availability zone's *shape*
//! (`us-west-1a`) but has no idea whether `us-west-1` is a region that
//! actually exists in this deployment, or whether it has an `a` zone.
//! [`RegionRegistry`] is the one place that knowledge lives.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::{AzId, RegionId};
use std::collections::BTreeMap;

const MAX_AZS_PER_REGION: u32 = 26; // one per lowercase letter suffix

/// A registered region and the availability zones under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionInfo {
    id: RegionId,
    azs: Vec<AzId>,
}

impl RegionInfo {
    pub fn id(&self) -> &RegionId {
        &self.id
    }

    pub fn azs(&self) -> &[AzId] {
        &self.azs
    }
}

/// A registry of regions and their availability zones.
#[derive(Debug, Clone, Default)]
pub struct RegionRegistry {
    regions: BTreeMap<RegionId, RegionInfo>,
}

impl RegionRegistry {
    pub fn new() -> Self {
        RegionRegistry {
            regions: BTreeMap::new(),
        }
    }

    /// Registers `region` with `az_count` availability zones, named
    /// `<region>a`, `<region>b`, ... in order.
    pub fn register(&mut self, region: RegionId, az_count: u32) -> Result<(), CloudError> {
        if self.regions.contains_key(&region) {
            return Err(CloudError::Conflict {
                what: "region",
                id: region.to_string(),
            });
        }
        if az_count == 0 || az_count > MAX_AZS_PER_REGION {
            return Err(CloudError::InvalidFormat {
                what: "availability zone count",
                value: az_count.to_string(),
                reason: format!("must be between 1 and {MAX_AZS_PER_REGION}"),
            });
        }
        let azs = (0..az_count)
            .map(|i| {
                let suffix = (b'a' + i as u8) as char;
                AzId::new(format!("{region}{suffix}")).expect("region + single letter is valid")
            })
            .collect();
        self.regions
            .insert(region.clone(), RegionInfo { id: region, azs });
        Ok(())
    }

    pub fn get(&self, region: &RegionId) -> Option<&RegionInfo> {
        self.regions.get(region)
    }

    /// Whether `az` belongs to a region that is both registered *and*
    /// actually has that zone.
    pub fn contains_az(&self, az: &AzId) -> bool {
        self.regions
            .get(&az.region())
            .is_some_and(|info| info.azs.contains(az))
    }

    /// Registered regions, in deterministic (sorted) order.
    pub fn regions(&self) -> impl Iterator<Item = &RegionInfo> {
        self.regions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(s: &str) -> RegionId {
        RegionId::new(s).unwrap()
    }

    fn az(s: &str) -> AzId {
        AzId::new(s).unwrap()
    }

    #[test]
    fn register_creates_the_expected_number_of_azs() {
        let mut reg = RegionRegistry::new();
        reg.register(region("us-west-1"), 3).unwrap();
        let info = reg.get(&region("us-west-1")).unwrap();
        assert_eq!(info.azs().len(), 3);
        assert_eq!(
            info.azs(),
            &[az("us-west-1a"), az("us-west-1b"), az("us-west-1c")]
        );
    }

    #[test]
    fn registering_the_same_region_twice_is_a_conflict() {
        let mut reg = RegionRegistry::new();
        reg.register(region("us-west-1"), 2).unwrap();
        assert!(matches!(
            reg.register(region("us-west-1"), 2),
            Err(CloudError::Conflict { .. })
        ));
    }

    #[test]
    fn rejects_zero_or_too_many_azs() {
        let mut reg = RegionRegistry::new();
        assert!(reg.register(region("us-west-1"), 0).is_err());
        assert!(reg
            .register(region("us-east-1"), MAX_AZS_PER_REGION + 1)
            .is_err());
    }

    #[test]
    fn contains_az_is_true_only_for_registered_zones() {
        let mut reg = RegionRegistry::new();
        reg.register(region("us-west-1"), 2).unwrap();
        assert!(reg.contains_az(&az("us-west-1a")));
        assert!(reg.contains_az(&az("us-west-1b")));
        // Shaped correctly, but the 3rd AZ was never registered.
        assert!(!reg.contains_az(&az("us-west-1c")));
        // A whole different, unregistered region.
        assert!(!reg.contains_az(&az("eu-central-1a")));
    }

    #[test]
    fn regions_are_iterated_in_sorted_order() {
        let mut reg = RegionRegistry::new();
        reg.register(region("us-west-1"), 1).unwrap();
        reg.register(region("ap-south-1"), 1).unwrap();
        reg.register(region("eu-central-1"), 1).unwrap();
        let ids: Vec<&RegionId> = reg.regions().map(RegionInfo::id).collect();
        assert_eq!(
            ids,
            vec![
                &region("ap-south-1"),
                &region("eu-central-1"),
                &region("us-west-1"),
            ]
        );
    }
}
