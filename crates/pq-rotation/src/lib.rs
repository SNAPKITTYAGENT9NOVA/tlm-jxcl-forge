//! Key-rotation policy: the `Active`/`DecryptOnly`/`Retired` lifecycle
//! and the `set_active`/`retire` transition rules, built on top of
//! `pq-keyring`'s generic, policy-agnostic storage.
//!
//! # `KeyRing`
//!
//! This crate defines the concrete, rotation-aware [`KeyRing`] type that
//! callers actually use: a thin wrapper around
//! `pq_keyring::KeyRing<KeyStatus>` that adds the real transition rules
//! ([`KeyRing::set_active`], [`KeyRing::retire`]) and the ring-level
//! sealing/opening convenience ([`KeyRing::seal`], [`KeyRing::open`])
//! that picks the right key automatically. `pq-keyring` itself stays
//! generic and knows nothing about what these statuses mean -- see its
//! crate docs.
#![forbid(unsafe_code)]

pub use pq_keyring::{DecapsulationKey, EncapsulationKey, Envelope, Error, KeyPair, SEED_LEN};

/// The lifecycle state of one [`KeyRing`] entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    /// Used for both sealing new values and opening existing ones. A
    /// ring should generally have at most one `Active` entry at a time
    /// (see [`KeyRing::set_active`]).
    Active,
    /// No longer used to seal new values, but still available to open
    /// values sealed under it before rotation -- the "grace period" of a
    /// rotation.
    DecryptOnly,
    /// No longer usable at all. [`KeyRing::open`] returns
    /// [`Error::RetiredKeyVersion`] for envelopes stamped with a retired
    /// version, rather than quietly failing to decrypt them, so an
    /// operator can distinguish "this key was deliberately retired" from
    /// "this envelope is corrupt."
    Retired,
}

/// A set of key pairs indexed by version, supporting key rotation:
/// exactly one version is [`KeyStatus::Active`] (used to seal new
/// values); older versions can be kept [`KeyStatus::DecryptOnly`] so
/// values already sealed remain readable until they naturally expire,
/// then [`KeyRing::retire`]d.
pub struct KeyRing {
    inner: pq_keyring::KeyRing<KeyStatus>,
}

impl KeyRing {
    pub fn new() -> Self {
        KeyRing {
            inner: pq_keyring::KeyRing::new(),
        }
    }

    /// Register `key_pair` under `version` with the given `status`.
    /// Overwrites any existing entry for that version.
    pub fn insert(&mut self, version: u32, key_pair: KeyPair, status: KeyStatus) {
        self.inner.insert(version, key_pair, status);
    }

    /// Change an existing entry's status in place, with no side effects
    /// on any other entry (e.g. `Active` -> `DecryptOnly` when rotating
    /// by hand, or `DecryptOnly` -> `Retired`). No-op if `version` isn't
    /// registered. For the usual rotation case, prefer
    /// [`KeyRing::set_active`], which also demotes the previous active
    /// entry.
    pub fn set_status(&mut self, version: u32, status: KeyStatus) {
        self.inner.set_status(version, status);
    }

    /// Convenience for rotation: mark `new_version` (already
    /// [`KeyRing::insert`]ed) as the sole [`KeyStatus::Active`] entry,
    /// demoting every other currently-`Active` entry to
    /// [`KeyStatus::DecryptOnly`] (never to `Retired` -- that's a
    /// separate, deliberate step via [`KeyRing::retire`] once old
    /// entries are truly no longer needed).
    pub fn set_active(&mut self, new_version: u32) -> Result<(), Error> {
        if self.inner.status(new_version).is_none() {
            return Err(Error::UnknownKeyVersion);
        }

        let currently_active: Vec<u32> = self
            .inner
            .iter()
            .filter(|(version, status)| *version != new_version && **status == KeyStatus::Active)
            .map(|(version, _)| version)
            .collect();
        for version in currently_active {
            self.inner.set_status(version, KeyStatus::DecryptOnly);
        }
        self.inner.set_status(new_version, KeyStatus::Active);
        Ok(())
    }

    /// Mark `version` [`KeyStatus::Retired`]. No-op if it isn't
    /// registered.
    pub fn retire(&mut self, version: u32) {
        self.inner.set_status(version, KeyStatus::Retired);
    }

    /// Seal `plaintext` under the ring's current active key, stamping
    /// the resulting envelope with that key's version.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Envelope, Error> {
        let (version, key_pair, _status) = self
            .inner
            .find(|status| *status == KeyStatus::Active)
            .ok_or(Error::NoActiveKeyVersion)?;
        pq_keyring::seal(&key_pair.encapsulation_key, version, plaintext)
    }

    /// Open `envelope` using whichever registered key matches its
    /// `key_version`, provided that key isn't [`KeyStatus::Retired`].
    pub fn open(&self, envelope: &Envelope) -> Result<Vec<u8>, Error> {
        let status = self
            .inner
            .status(envelope.key_version)
            .ok_or(Error::UnknownKeyVersion)?;
        if *status == KeyStatus::Retired {
            return Err(Error::RetiredKeyVersion);
        }
        let key_pair = self
            .inner
            .key_pair(envelope.key_version)
            .expect("status lookup above already confirmed this version is registered");
        pq_keyring::open(&key_pair.decapsulation_key, envelope)
    }
}

impl Default for KeyRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_ring_seals_under_the_active_version() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), KeyStatus::Active);
        let envelope = ring.seal(b"hello").unwrap();
        assert_eq!(envelope.key_version, 1);
        assert_eq!(ring.open(&envelope).unwrap(), b"hello");
    }

    #[test]
    fn key_ring_with_no_active_entry_refuses_to_seal() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), KeyStatus::DecryptOnly);
        assert_eq!(ring.seal(b"hello"), Err(Error::NoActiveKeyVersion));
    }

    /// The crate's most important test: a full rotation scenario. Seal
    /// under v1, rotate to v2 (v1 automatically demotes), confirm both
    /// versions are still readable during the grace period, retire v1,
    /// confirm v1 now fails closed while v2 remains completely
    /// unaffected.
    #[test]
    fn rotation_keeps_old_entries_readable_until_retired() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), KeyStatus::Active);
        let old_envelope = ring.seal(b"sealed under v1").unwrap();

        // Rotate: v2 becomes active, v1 automatically demotes to DecryptOnly.
        ring.insert(2, KeyPair::generate(), KeyStatus::Active);
        ring.set_active(2).unwrap();

        let new_envelope = ring.seal(b"sealed under v2").unwrap();
        assert_eq!(new_envelope.key_version, 2);

        // Both old and new entries are still readable during the grace period.
        assert_eq!(ring.open(&old_envelope).unwrap(), b"sealed under v1");
        assert_eq!(ring.open(&new_envelope).unwrap(), b"sealed under v2");

        // Once retired, the old version can no longer decrypt anything,
        // but the new version is unaffected.
        ring.retire(1);
        assert_eq!(ring.open(&old_envelope), Err(Error::RetiredKeyVersion));
        assert_eq!(ring.open(&new_envelope).unwrap(), b"sealed under v2");
    }

    #[test]
    fn unknown_key_version_is_reported_distinctly_from_retired() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), KeyStatus::Active);
        let envelope = ring.seal(b"x").unwrap();

        let mut empty_ring = KeyRing::new();
        empty_ring.insert(99, KeyPair::generate(), KeyStatus::Active);
        assert_eq!(empty_ring.open(&envelope), Err(Error::UnknownKeyVersion));
    }

    #[test]
    fn set_active_rejects_unregistered_version() {
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), KeyStatus::Active);
        assert_eq!(ring.set_active(2), Err(Error::UnknownKeyVersion));
    }

    #[test]
    fn set_active_demotes_every_other_active_entry() {
        // Guards against a rotation policy that only demotes "the"
        // active entry and misses a second one if the ring somehow ends
        // up with more than one Active at a time.
        let mut ring = KeyRing::new();
        ring.insert(1, KeyPair::generate(), KeyStatus::Active);
        ring.insert(2, KeyPair::generate(), KeyStatus::Active);
        ring.insert(3, KeyPair::generate(), KeyStatus::Active);
        ring.set_active(3).unwrap();

        let statuses: Vec<(u32, KeyStatus)> = ring.inner.iter().map(|(v, s)| (v, *s)).collect();
        assert_eq!(
            statuses,
            vec![
                (1, KeyStatus::DecryptOnly),
                (2, KeyStatus::DecryptOnly),
                (3, KeyStatus::Active),
            ]
        );
    }

    #[test]
    fn retire_on_unregistered_version_is_a_no_op() {
        let mut ring: KeyRing = KeyRing::new();
        ring.retire(42);
        assert!(ring.inner.status(42).is_none());
    }
}
