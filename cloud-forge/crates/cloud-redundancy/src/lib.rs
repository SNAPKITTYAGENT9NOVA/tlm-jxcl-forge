// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! How durable a piece of stored data is: how many independent copies
//! or erasure-coded shards it's split into, how many of those must
//! survive to reconstruct it, and how many losses that tolerates.
//!
//! This is the one shared vocabulary every future storage-shaped
//! resource (an object, a volume, a database's write-ahead log) would
//! otherwise redefine on its own -- exactly the kind of concept this
//! workspace's non-negotiable rule asks to be built once.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;

/// A durability scheme for stored data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedundancyScheme {
    /// `copies` full, independent copies of the data. Any single
    /// surviving copy is enough to reconstruct it.
    Replicated { copies: u32 },
    /// `data_shards` shards of original data plus `parity_shards`
    /// shards of parity, such that any `data_shards` of the combined
    /// `data_shards + parity_shards` shards reconstruct the original
    /// (the property real erasure codes, e.g. Reed-Solomon, provide --
    /// this crate models the resulting arithmetic, not an encoder).
    ErasureCoded {
        data_shards: u32,
        parity_shards: u32,
    },
}

impl RedundancyScheme {
    pub fn replicated(copies: u32) -> Result<Self, CloudError> {
        if copies == 0 {
            return Err(CloudError::InvalidFormat {
                what: "redundancy scheme",
                value: "0 copies".to_string(),
                reason: "a replicated scheme needs at least one copy".to_string(),
            });
        }
        Ok(RedundancyScheme::Replicated { copies })
    }

    pub fn erasure_coded(data_shards: u32, parity_shards: u32) -> Result<Self, CloudError> {
        if data_shards == 0 {
            return Err(CloudError::InvalidFormat {
                what: "redundancy scheme",
                value: "0 data shards".to_string(),
                reason: "an erasure-coded scheme needs at least one data shard".to_string(),
            });
        }
        if parity_shards == 0 {
            return Err(CloudError::InvalidFormat {
                what: "redundancy scheme",
                value: "0 parity shards".to_string(),
                reason: "an erasure-coded scheme with no parity shards provides no redundancy -- use Replicated with 1 copy instead".to_string(),
            });
        }
        Ok(RedundancyScheme::ErasureCoded {
            data_shards,
            parity_shards,
        })
    }

    /// The total number of independent shards data is split into.
    pub fn total_shards(&self) -> u32 {
        match self {
            RedundancyScheme::Replicated { copies } => *copies,
            RedundancyScheme::ErasureCoded {
                data_shards,
                parity_shards,
            } => data_shards + parity_shards,
        }
    }

    /// The minimum number of shards that must remain available to
    /// reconstruct the original data.
    pub fn shards_required_to_reconstruct(&self) -> u32 {
        match self {
            RedundancyScheme::Replicated { .. } => 1,
            RedundancyScheme::ErasureCoded { data_shards, .. } => *data_shards,
        }
    }

    /// How many shard losses this scheme tolerates before data
    /// becomes unrecoverable.
    pub fn max_tolerable_losses(&self) -> u32 {
        self.total_shards() - self.shards_required_to_reconstruct()
    }

    /// Whether `available_shards` surviving shards are enough to
    /// reconstruct the original data.
    pub fn can_reconstruct_from(&self, available_shards: u32) -> bool {
        available_shards >= self.shards_required_to_reconstruct()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replicated_rejects_zero_copies() {
        assert!(RedundancyScheme::replicated(0).is_err());
    }

    #[test]
    fn replicated_needs_only_one_surviving_copy() {
        let scheme = RedundancyScheme::replicated(3).unwrap();
        assert_eq!(scheme.shards_required_to_reconstruct(), 1);
        assert_eq!(scheme.total_shards(), 3);
        assert_eq!(scheme.max_tolerable_losses(), 2);
    }

    #[test]
    fn replicated_can_reconstruct_from_exactly_one_shard_but_not_zero() {
        let scheme = RedundancyScheme::replicated(3).unwrap();
        assert!(scheme.can_reconstruct_from(1));
        assert!(!scheme.can_reconstruct_from(0));
    }

    #[test]
    fn erasure_coded_rejects_zero_data_shards() {
        assert!(RedundancyScheme::erasure_coded(0, 2).is_err());
    }

    #[test]
    fn erasure_coded_rejects_zero_parity_shards() {
        assert!(RedundancyScheme::erasure_coded(4, 0).is_err());
    }

    #[test]
    fn erasure_coded_needs_exactly_data_shards_to_reconstruct() {
        let scheme = RedundancyScheme::erasure_coded(4, 2).unwrap();
        assert_eq!(scheme.shards_required_to_reconstruct(), 4);
        assert_eq!(scheme.total_shards(), 6);
        assert_eq!(scheme.max_tolerable_losses(), 2);
    }

    #[test]
    fn erasure_coded_reconstruction_threshold_is_a_strict_boundary() {
        let scheme = RedundancyScheme::erasure_coded(4, 2).unwrap();
        assert!(scheme.can_reconstruct_from(4));
        assert!(!scheme.can_reconstruct_from(3));
        assert!(scheme.can_reconstruct_from(6));
    }

    #[test]
    fn schemes_with_the_same_total_shards_can_tolerate_different_losses() {
        // 3x replication and 1-data/2-parity erasure coding both use 3
        // total shards, but tolerate a different number of losses.
        let replicated = RedundancyScheme::replicated(3).unwrap();
        let erasure_coded = RedundancyScheme::erasure_coded(1, 2).unwrap();
        assert_eq!(replicated.total_shards(), erasure_coded.total_shards());
        assert_eq!(replicated.max_tolerable_losses(), 2);
        assert_eq!(erasure_coded.max_tolerable_losses(), 2);
    }

    #[test]
    fn a_single_replica_tolerates_no_losses_at_all() {
        let scheme = RedundancyScheme::replicated(1).unwrap();
        assert_eq!(scheme.max_tolerable_losses(), 0);
        assert!(!scheme.can_reconstruct_from(0));
    }
}
