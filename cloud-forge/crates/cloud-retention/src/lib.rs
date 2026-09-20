// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Which of a database-shaped resource's backups/snapshots are
//! eligible for deletion.
//!
//! A [`RetentionPolicy`] combines two rules that both have to agree
//! before something is deleted: it must be older than the configured
//! age window, **and** it must not be among the `min_to_keep` most
//! recent snapshots -- so a policy can never delete every snapshot at
//! once just because all of them happen to be old (e.g. after a long
//! period with no new writes). Age alone is not the rule; the floor
//! matters too.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::{ResourceId, Timestamp};
use std::collections::HashSet;

/// A snapshot-retention policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    max_age_millis: u64,
    min_to_keep: u32,
}

impl RetentionPolicy {
    pub fn new(max_age_millis: u64, min_to_keep: u32) -> Result<Self, CloudError> {
        if max_age_millis == 0 {
            return Err(CloudError::InvalidFormat {
                what: "retention policy",
                value: "0ms".to_string(),
                reason: "max age must be greater than zero".to_string(),
            });
        }
        Ok(RetentionPolicy {
            max_age_millis,
            min_to_keep,
        })
    }

    pub fn max_age_millis(&self) -> u64 {
        self.max_age_millis
    }

    pub fn min_to_keep(&self) -> u32 {
        self.min_to_keep
    }

    /// Whether something created at `created_at` counts as expired at
    /// `now` under this policy's age window alone (the floor in
    /// [`Self::eligible_for_deletion`] is not applied here).
    pub fn is_expired(&self, created_at: Timestamp, now: Timestamp) -> bool {
        now.as_millis().saturating_sub(created_at.as_millis()) >= self.max_age_millis
    }

    /// Given the current time and every snapshot's id and creation
    /// time, returns the ids eligible for deletion: expired by age,
    /// and not among the `min_to_keep` most recently created.
    pub fn eligible_for_deletion(
        &self,
        now: Timestamp,
        snapshots: &[(ResourceId, Timestamp)],
    ) -> Vec<ResourceId> {
        let mut by_recency: Vec<&(ResourceId, Timestamp)> = snapshots.iter().collect();
        by_recency.sort_by(|a, b| b.1.cmp(&a.1));
        let protected: HashSet<&ResourceId> = by_recency
            .iter()
            .take(self.min_to_keep as usize)
            .map(|(id, _)| id)
            .collect();

        snapshots
            .iter()
            .filter(|(id, created_at)| !protected.contains(id) && self.is_expired(*created_at, now))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    fn ms(millis: u64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    #[test]
    fn rejects_a_zero_max_age() {
        assert!(RetentionPolicy::new(0, 1).is_err());
    }

    #[test]
    fn nothing_is_expired_before_the_window_elapses() {
        let policy = RetentionPolicy::new(1000, 0).unwrap();
        assert!(!policy.is_expired(ms(0), ms(999)));
    }

    #[test]
    fn something_is_expired_at_exactly_the_window_boundary() {
        let policy = RetentionPolicy::new(1000, 0).unwrap();
        assert!(policy.is_expired(ms(0), ms(1000)));
    }

    #[test]
    fn eligible_for_deletion_returns_only_expired_snapshots_beyond_the_floor() {
        let policy = RetentionPolicy::new(1000, 1).unwrap();
        let snapshots = vec![
            (id("snap-old"), ms(0)),   // expired, and not the most recent
            (id("snap-new"), ms(950)), // not expired yet
        ];
        let eligible = policy.eligible_for_deletion(ms(1000), &snapshots);
        assert_eq!(eligible, vec![id("snap-old")]);
    }

    #[test]
    fn the_min_to_keep_most_recent_snapshots_are_protected_even_if_expired() {
        let policy = RetentionPolicy::new(1000, 2).unwrap();
        // All three are expired by age, but only 1 may be deleted --
        // the two most recent are protected by the floor.
        let snapshots = vec![
            (id("snap-1"), ms(0)),
            (id("snap-2"), ms(100)),
            (id("snap-3"), ms(200)),
        ];
        let eligible = policy.eligible_for_deletion(ms(10_000), &snapshots);
        assert_eq!(eligible, vec![id("snap-1")]);
    }

    #[test]
    fn fewer_snapshots_than_the_floor_means_nothing_is_eligible() {
        let policy = RetentionPolicy::new(1000, 5).unwrap();
        let snapshots = vec![(id("snap-1"), ms(0)), (id("snap-2"), ms(100))];
        let eligible = policy.eligible_for_deletion(ms(10_000), &snapshots);
        assert!(eligible.is_empty());
    }

    #[test]
    fn a_zero_floor_makes_every_expired_snapshot_eligible() {
        let policy = RetentionPolicy::new(1000, 0).unwrap();
        let snapshots = vec![(id("snap-1"), ms(0)), (id("snap-2"), ms(100))];
        let mut eligible = policy.eligible_for_deletion(ms(10_000), &snapshots);
        eligible.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        assert_eq!(eligible, vec![id("snap-1"), id("snap-2")]);
    }

    #[test]
    fn no_snapshots_that_are_still_within_the_age_window_are_ever_eligible() {
        let policy = RetentionPolicy::new(1000, 0).unwrap();
        let snapshots = vec![(id("snap-1"), ms(9500))];
        let eligible = policy.eligible_for_deletion(ms(10_000), &snapshots);
        assert!(eligible.is_empty());
    }
}
