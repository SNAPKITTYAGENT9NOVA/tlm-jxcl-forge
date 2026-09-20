// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The `KeyRing` data structure: an indexed set of key-pair entries.
//!
//! # Scope
//!
//! This crate owns storage and lookup only -- insert an entry under a
//! version number, read/overwrite its status, look one up by version or
//! by a predicate over its status, iterate all of them. It deliberately
//! knows nothing about what a "status" *means*: [`KeyRing`] is generic
//! over the status type `S`, so it has no notion of "active" or
//! "retired" and enforces no transition rules between them. That's
//! `pq-rotation`'s job -- see its crate docs for the concrete
//! `KeyStatus` type and the real `Active -> DecryptOnly -> Retired`
//! lifecycle built on top of the generic interface here. Keeping this
//! split means a future alternate rotation policy (e.g. time-based
//! auto-retirement, using some other status type entirely) could be
//! built without ever touching this crate.
#![forbid(unsafe_code)]

pub use pq_envelope::{
    open, seal, DecapsulationKey, EncapsulationKey, Envelope, Error, KeyPair, SEED_LEN,
};

use std::collections::BTreeMap;

struct KeyRingEntry<S> {
    key_pair: KeyPair,
    status: S,
}

/// A set of key pairs indexed by a `u32` version number, each carrying an
/// arbitrary status value of type `S`. See the module docs for why this
/// crate doesn't interpret `S` itself.
pub struct KeyRing<S> {
    entries: BTreeMap<u32, KeyRingEntry<S>>,
}

impl<S> KeyRing<S> {
    pub fn new() -> Self {
        KeyRing {
            entries: BTreeMap::new(),
        }
    }

    /// Register `key_pair` under `version` with the given `status`.
    /// Overwrites any existing entry for that version.
    pub fn insert(&mut self, version: u32, key_pair: KeyPair, status: S) {
        self.entries
            .insert(version, KeyRingEntry { key_pair, status });
    }

    /// Whether an entry is registered under `version`.
    pub fn contains(&self, version: u32) -> bool {
        self.entries.contains_key(&version)
    }

    /// The number of registered entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entries are registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The status registered for `version`, if any.
    pub fn status(&self, version: u32) -> Option<&S> {
        self.entries.get(&version).map(|e| &e.status)
    }

    /// The key pair registered for `version`, if any.
    pub fn key_pair(&self, version: u32) -> Option<&KeyPair> {
        self.entries.get(&version).map(|e| &e.key_pair)
    }

    /// Overwrite the status of an existing entry in place. No-op if
    /// `version` isn't registered. This is a pure, policy-free mutator:
    /// it does not look at, or change, any other entry.
    pub fn set_status(&mut self, version: u32, status: S) {
        if let Some(entry) = self.entries.get_mut(&version) {
            entry.status = status;
        }
    }

    /// Iterate over every registered `(version, status)` pair, in
    /// ascending version order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &S)> {
        self.entries.iter().map(|(v, e)| (*v, &e.status))
    }

    /// Find the first entry (in ascending version order) whose status
    /// matches `predicate`, returning its version, key pair, and status.
    pub fn find(&self, predicate: impl Fn(&S) -> bool) -> Option<(u32, &KeyPair, &S)> {
        self.entries
            .iter()
            .find(|(_, e)| predicate(&e.status))
            .map(|(v, e)| (*v, &e.key_pair, &e.status))
    }
}

impl<S> Default for KeyRing<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal stand-in status type, deliberately with no notion of
    /// "active"/"retired" -- proof that this crate's storage/lookup
    /// really is policy-agnostic, independent of `pq-rotation`'s
    /// `KeyStatus`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestStatus {
        Fresh,
        Stale,
    }

    #[test]
    fn new_ring_is_empty() {
        let ring: KeyRing<TestStatus> = KeyRing::new();
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
        assert!(!ring.contains(1));
    }

    #[test]
    fn insert_then_lookup_roundtrips() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), TestStatus::Fresh);
        assert!(ring.contains(1));
        assert_eq!(ring.len(), 1);
        assert_eq!(ring.status(1), Some(&TestStatus::Fresh));
        assert!(ring.key_pair(1).is_some());
    }

    #[test]
    fn missing_version_returns_none() {
        let ring: KeyRing<TestStatus> = KeyRing::new();
        assert_eq!(ring.status(99), None);
        assert!(ring.key_pair(99).is_none());
    }

    #[test]
    fn insert_overwrites_existing_version() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), TestStatus::Fresh);
        ring.insert(1, KeyPair::generate(), TestStatus::Stale);
        assert_eq!(ring.len(), 1);
        assert_eq!(ring.status(1), Some(&TestStatus::Stale));
    }

    #[test]
    fn set_status_updates_only_the_named_entry() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), TestStatus::Fresh);
        ring.insert(2, KeyPair::generate(), TestStatus::Fresh);
        ring.set_status(1, TestStatus::Stale);
        assert_eq!(ring.status(1), Some(&TestStatus::Stale));
        assert_eq!(ring.status(2), Some(&TestStatus::Fresh));
    }

    #[test]
    fn set_status_on_unregistered_version_is_a_no_op() {
        let mut ring: KeyRing<TestStatus> = KeyRing::new();
        ring.set_status(5, TestStatus::Stale);
        assert!(!ring.contains(5));
    }

    #[test]
    fn iter_visits_every_entry_in_ascending_version_order() {
        let mut ring = KeyRing::new();
        ring.insert(3, KeyPair::generate(), TestStatus::Stale);
        ring.insert(1, KeyPair::generate(), TestStatus::Fresh);
        ring.insert(2, KeyPair::generate(), TestStatus::Fresh);
        let versions: Vec<u32> = ring.iter().map(|(v, _)| v).collect();
        assert_eq!(versions, vec![1, 2, 3]);
    }

    #[test]
    fn find_returns_first_match_by_predicate() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), TestStatus::Stale);
        ring.insert(2, KeyPair::generate(), TestStatus::Fresh);
        let found = ring.find(|s| *s == TestStatus::Fresh);
        assert_eq!(found.map(|(v, _, s)| (v, *s)), Some((2, TestStatus::Fresh)));
    }

    #[test]
    fn find_returns_none_when_nothing_matches() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), TestStatus::Stale);
        assert!(ring.find(|s| *s == TestStatus::Fresh).is_none());
    }

    #[test]
    fn default_ring_is_empty() {
        let ring: KeyRing<TestStatus> = Default::default();
        assert!(ring.is_empty());
    }
}
