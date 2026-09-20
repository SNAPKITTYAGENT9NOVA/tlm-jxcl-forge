// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Enforces that a database-shaped resource's schema migrations apply
//! strictly sequentially and without gaps.
//!
//! A [`MigrationLedger`] doesn't run migrations -- it has no idea what
//! SQL or transformation a version number corresponds to -- it owns
//! exactly one invariant: version `N` can only be recorded as applied
//! once version `N-1` already is, starting from `1`. Skipping ahead,
//! re-applying an already-applied version, and applying version `0`
//! are all rejected the same way, and a rejection never mutates the
//! ledger.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;

/// The sequence of schema-migration versions applied so far.
#[derive(Debug, Clone, Default)]
pub struct MigrationLedger {
    applied: Vec<u32>,
}

impl MigrationLedger {
    pub fn new() -> Self {
        MigrationLedger {
            applied: Vec::new(),
        }
    }

    /// The highest applied migration version, or `0` if none have
    /// been applied yet.
    pub fn current_version(&self) -> u32 {
        self.applied.last().copied().unwrap_or(0)
    }

    /// Records `version` as applied. Rejected (leaving the ledger
    /// completely unchanged) unless `version` is exactly one more
    /// than [`Self::current_version`].
    pub fn apply(&mut self, version: u32) -> Result<(), CloudError> {
        let expected = self.current_version() + 1;
        if version != expected {
            return Err(CloudError::InvalidTransition {
                what: "migration",
                from: self.current_version().to_string(),
                to: version.to_string(),
            });
        }
        self.applied.push(version);
        Ok(())
    }

    pub fn is_applied(&self, version: u32) -> bool {
        self.applied.contains(&version)
    }

    /// Every applied version, in the (necessarily increasing) order
    /// it was applied.
    pub fn applied_versions(&self) -> &[u32] {
        &self.applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_ledger_starts_at_version_zero() {
        let ledger = MigrationLedger::new();
        assert_eq!(ledger.current_version(), 0);
        assert!(ledger.applied_versions().is_empty());
    }

    #[test]
    fn migrations_must_start_at_version_one() {
        let mut ledger = MigrationLedger::new();
        assert!(ledger.apply(2).is_err());
        assert_eq!(ledger.current_version(), 0);
        ledger.apply(1).unwrap();
        assert_eq!(ledger.current_version(), 1);
    }

    #[test]
    fn sequential_migrations_apply_in_order() {
        let mut ledger = MigrationLedger::new();
        ledger.apply(1).unwrap();
        ledger.apply(2).unwrap();
        ledger.apply(3).unwrap();
        assert_eq!(ledger.current_version(), 3);
        assert_eq!(ledger.applied_versions(), &[1, 2, 3]);
    }

    #[test]
    fn skipping_a_version_is_rejected() {
        let mut ledger = MigrationLedger::new();
        ledger.apply(1).unwrap();
        let err = ledger.apply(3).unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert_eq!(ledger.current_version(), 1);
    }

    #[test]
    fn reapplying_an_already_applied_version_is_rejected() {
        let mut ledger = MigrationLedger::new();
        ledger.apply(1).unwrap();
        let err = ledger.apply(1).unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert_eq!(ledger.applied_versions(), &[1]);
    }

    #[test]
    fn a_rejected_apply_leaves_the_ledger_completely_unchanged() {
        let mut ledger = MigrationLedger::new();
        ledger.apply(1).unwrap();
        ledger.apply(2).unwrap();
        assert!(ledger.apply(10).is_err());
        assert_eq!(ledger.current_version(), 2);
        assert_eq!(ledger.applied_versions(), &[1, 2]);
    }

    #[test]
    fn is_applied_reflects_ledger_state() {
        let mut ledger = MigrationLedger::new();
        ledger.apply(1).unwrap();
        assert!(ledger.is_applied(1));
        assert!(!ledger.is_applied(2));
    }
}
