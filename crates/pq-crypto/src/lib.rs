// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Post-quantum envelope encryption for at-rest cache values.
//!
//! # Construction
//!
//! This is a standard hybrid public-key encryption (KEM/DEM) scheme:
//!
//! 1. **KEM**: [ML-KEM-768][fips203] (NIST FIPS 203, formerly known as
//!    CRYSTALS-Kyber) encapsulates a fresh 32-byte shared secret to the
//!    holder of a [`DecapsulationKey`], producing a [`KemCiphertext`].
//!    ML-KEM is believed secure against an attacker holding a
//!    cryptographically relevant quantum computer, unlike classical
//!    key-exchange schemes (RSA, ECDH) that Shor's algorithm breaks.
//! 2. **KDF**: the shared secret is expanded via HKDF-SHA256 with a
//!    fixed, versioned info string, into a 256-bit AES key. This domain
//!    separation means the same shared secret could not accidentally be
//!    reused as a key for a different purpose.
//! 3. **DEM**: the actual plaintext is encrypted with AES-256-GCM under
//!    that derived key and a fresh random 96-bit nonce.
//!
//! The [`Envelope`] bundles the KEM ciphertext, nonce, and AEAD
//! ciphertext together into one self-describing byte blob suitable for
//! storing as an opaque value in a cache (see the `pq-cache` crate).
//!
//! # Threat model
//!
//! This protects the *confidentiality of cached values at rest* against
//! anyone who can read the cache storage (e.g. a compromised or
//! untrusted Redis instance/operator) but does not hold the
//! [`DecapsulationKey`] — including an attacker with a quantum computer.
//! It does **not** by itself secure the network transport between the
//! application and the cache (see `docs/HARDENING.md` for the full
//! scope discussion); that is a separate, orthogonal concern (e.g.
//! TLS/mTLS on the Redis connection).
//!
//! [fips203]: https://csrc.nist.gov/pubs/fips/203/final
#![forbid(unsafe_code)]

use aes_gcm::{
    aead::{Aead, Generate, KeyInit, Nonce},
    Aes256Gcm, Key,
};
use hkdf::Hkdf;
use kem::{Decapsulate, Encapsulate};
use ml_kem::MlKem768;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;

pub type DecapsulationKey = <MlKem768 as kem::Kem>::DecapsulationKey;
pub type EncapsulationKey = <MlKem768 as kem::Kem>::EncapsulationKey;

/// Domain-separation string for the HKDF expand step. Bumping the
/// version suffix is a deliberate, documented way to invalidate all
/// previously-sealed envelopes if the construction ever changes.
const HKDF_INFO: &[u8] = b"pq-crypto/aes-256-gcm-key/v1";

const NONCE_LEN: usize = 12;

/// A key pair for this scheme. Generate one per service instance; there
/// is no need to persist it across restarts (see crate docs and
/// `pq-cache`'s handling of undecryptable envelopes as cache misses).
pub struct KeyPair {
    pub decapsulation_key: DecapsulationKey,
    pub encapsulation_key: EncapsulationKey,
}

/// Length in bytes of the seed accepted by [`KeyPair::from_seed`].
pub const SEED_LEN: usize = 64;

impl KeyPair {
    /// Generate a fresh key pair using the operating system's CSPRNG.
    ///
    /// Note: a randomly generated key pair is process-local. If multiple
    /// service replicas each call `generate()` independently, none of
    /// them can decrypt entries another replica sealed — see
    /// [`KeyPair::from_seed`] for the fix when that matters (see also
    /// `docs/HARDENING.md`).
    pub fn generate() -> Self {
        use kem::Kem;
        let (decapsulation_key, encapsulation_key) = MlKem768::generate_keypair();
        KeyPair {
            decapsulation_key,
            encapsulation_key,
        }
    }

    /// Deterministically derive a key pair from a 64-byte seed. Every
    /// replica of a service that is given the *same* seed (e.g. via a
    /// shared secret/env var) will derive the *same* key pair, and so
    /// can decrypt each other's cache entries. The seed itself must be
    /// kept confidential with the same rigor as a decapsulation key --
    /// anyone who has it can decrypt everything sealed under it.
    pub fn from_seed(seed: &[u8; SEED_LEN]) -> Self {
        use kem::FromSeed;
        let seed_array = ml_kem::Seed::from(*seed);
        let (decapsulation_key, encapsulation_key) = MlKem768::from_seed(&seed_array);
        KeyPair {
            decapsulation_key,
            encapsulation_key,
        }
    }
}

/// A sealed value: which key version sealed it, plus the KEM ciphertext,
/// AEAD nonce, and AEAD ciphertext, serializable to and from a single
/// opaque byte blob.
///
/// `key_version` is what makes key rotation possible (spec: see
/// `docs/HARDENING.md` "Key rotation"): a [`KeyRing`] stamps every
/// envelope with the version that sealed it, so a later `open()` against
/// a *different* active version can still find and use the right
/// (now decrypt-only) key for entries sealed before the rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub key_version: u32,
    pub kem_ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub aead_ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// HKDF was asked to expand to an invalid output length (should be
    /// unreachable for the fixed 32-byte AES-256 key this crate derives).
    KeyDerivation,
    /// AEAD encryption failed (the `aes-gcm` crate deliberately gives no
    /// further detail, to avoid leaking side-channel information).
    Encrypt,
    /// AEAD decryption/authentication failed: wrong key, tampered
    /// ciphertext, or a foreign/corrupted envelope.
    Decrypt,
    /// The envelope's KEM ciphertext was the wrong length for this KEM's
    /// parameter set.
    InvalidKemCiphertext,
    /// The envelope's nonce was not exactly 12 bytes.
    InvalidNonceLength,
    /// A serialized envelope was truncated or malformed.
    MalformedEnvelope,
    /// A [`KeyRing`] has no key registered for the envelope's declared
    /// `key_version`.
    UnknownKeyVersion,
    /// The [`KeyRing`] key for this envelope's `key_version` has been
    /// [`KeyRing::retire`]d and is no longer available for decryption.
    RetiredKeyVersion,
    /// [`KeyRing::seal`] was called on a ring with no
    /// [`KeyStatus::Active`] entry to seal new values under.
    NoActiveKeyVersion,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

fn derive_aes_key(shared_secret: &[u8]) -> Result<Key<Aes256Gcm>, Error> {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut okm = [0u8; 32];
    hk.expand(HKDF_INFO, &mut okm)
        .map_err(|_| Error::KeyDerivation)?;
    Ok(Key::<Aes256Gcm>::from(okm))
}

/// Encrypt `plaintext` so that only the holder of the matching
/// [`DecapsulationKey`] can recover it, stamping the envelope with
/// `key_version` (see [`KeyRing`] for the usual, rotation-aware way to
/// call this rather than tracking versions by hand).
pub fn seal(ek: &EncapsulationKey, key_version: u32, plaintext: &[u8]) -> Result<Envelope, Error> {
    let (kem_ciphertext, shared_secret) = ek.encapsulate();
    let key = derive_aes_key(&shared_secret)?;
    let cipher = Aes256Gcm::new(&key);

    let nonce = Nonce::<Aes256Gcm>::generate();
    let aead_ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| Error::Encrypt)?;

    Ok(Envelope {
        key_version,
        kem_ciphertext: kem_ciphertext.to_vec(),
        nonce: nonce.to_vec(),
        aead_ciphertext,
    })
}

/// Decrypt an [`Envelope`] previously produced by [`seal`] for the
/// matching [`EncapsulationKey`]. Callers that manage multiple key
/// versions should generally use [`KeyRing::open`] instead, which picks
/// the right `dk` for the envelope's `key_version` automatically.
pub fn open(dk: &DecapsulationKey, envelope: &Envelope) -> Result<Vec<u8>, Error> {
    if envelope.nonce.len() != NONCE_LEN {
        return Err(Error::InvalidNonceLength);
    }
    let shared_secret = dk
        .decapsulate_slice(&envelope.kem_ciphertext)
        .map_err(|_| Error::InvalidKemCiphertext)?;
    let key = derive_aes_key(&shared_secret)?;
    let cipher = Aes256Gcm::new(&key);

    let nonce = Nonce::<Aes256Gcm>::try_from(envelope.nonce.as_slice())
        .map_err(|_| Error::InvalidNonceLength)?;
    cipher
        .decrypt(&nonce, envelope.aead_ciphertext.as_ref())
        .map_err(|_| Error::Decrypt)
}

impl Envelope {
    /// Serialize to a single self-describing byte blob:
    /// `[key_version: u32 LE][kem_len: u32 LE][kem_ciphertext][nonce_len: u32 LE][nonce][aead_ciphertext]`.
    /// Length-prefixing (rather than hardcoding the KEM's fixed
    /// ciphertext size) keeps this format independent of which ML-KEM
    /// parameter set is in use.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            4 + 4 + self.kem_ciphertext.len() + 4 + self.nonce.len() + self.aead_ciphertext.len(),
        );
        out.extend_from_slice(&self.key_version.to_le_bytes());
        out.extend_from_slice(&(self.kem_ciphertext.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.kem_ciphertext);
        out.extend_from_slice(&(self.nonce.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.aead_ciphertext);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut pos = 0usize;
        let take = |pos: &mut usize, n: usize, bytes: &[u8]| -> Result<Vec<u8>, Error> {
            let end = pos.checked_add(n).ok_or(Error::MalformedEnvelope)?;
            let slice = bytes.get(*pos..end).ok_or(Error::MalformedEnvelope)?;
            *pos = end;
            Ok(slice.to_vec())
        };
        let read_u32 = |pos: &mut usize, bytes: &[u8]| -> Result<u32, Error> {
            let raw = take(pos, 4, bytes)?;
            Ok(u32::from_le_bytes(raw.try_into().unwrap()))
        };

        let key_version = read_u32(&mut pos, bytes)?;
        let kem_len = read_u32(&mut pos, bytes)? as usize;
        let kem_ciphertext = take(&mut pos, kem_len, bytes)?;
        let nonce_len = read_u32(&mut pos, bytes)? as usize;
        let nonce = take(&mut pos, nonce_len, bytes)?;
        let aead_ciphertext = bytes.get(pos..).ok_or(Error::MalformedEnvelope)?.to_vec();

        Ok(Envelope {
            key_version,
            kem_ciphertext,
            nonce,
            aead_ciphertext,
        })
    }
}

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
    /// operator can distinguish "this key was deliberately retired"
    /// from "this envelope is corrupt."
    Retired,
}

struct KeyRingEntry {
    key_pair: KeyPair,
    status: KeyStatus,
}

/// A set of key pairs indexed by version, supporting key rotation:
/// exactly one version is [`KeyStatus::Active`] (used to seal new
/// values); older versions can be kept [`KeyStatus::DecryptOnly`] so
/// values already in the cache remain readable until they naturally
/// expire, then [`KeyRing::retire`]d.
#[derive(Default)]
pub struct KeyRing {
    entries: BTreeMap<u32, KeyRingEntry>,
}

impl KeyRing {
    pub fn new() -> Self {
        KeyRing {
            entries: BTreeMap::new(),
        }
    }

    /// Register `key_pair` under `version` with the given `status`.
    /// Overwrites any existing entry for that version.
    pub fn insert(&mut self, version: u32, key_pair: KeyPair, status: KeyStatus) {
        self.entries
            .insert(version, KeyRingEntry { key_pair, status });
    }

    /// Change an existing entry's status in place (e.g. `Active` ->
    /// `DecryptOnly` when rotating, or `DecryptOnly` -> `Retired` once a
    /// rotation's grace period has passed). No-op if `version` isn't
    /// registered.
    pub fn set_status(&mut self, version: u32, status: KeyStatus) {
        if let Some(entry) = self.entries.get_mut(&version) {
            entry.status = status;
        }
    }

    /// Convenience for rotation: mark `new_version` (already
    /// [`KeyRing::insert`]ed) as the sole [`KeyStatus::Active`] entry,
    /// demoting every other currently-`Active` entry to
    /// [`KeyStatus::DecryptOnly`] (never to `Retired` -- that's a
    /// separate, deliberate step once old entries are truly no longer
    /// needed).
    pub fn set_active(&mut self, new_version: u32) -> Result<(), Error> {
        if !self.entries.contains_key(&new_version) {
            return Err(Error::UnknownKeyVersion);
        }
        for (version, entry) in self.entries.iter_mut() {
            if *version == new_version {
                entry.status = KeyStatus::Active;
            } else if entry.status == KeyStatus::Active {
                entry.status = KeyStatus::DecryptOnly;
            }
        }
        Ok(())
    }

    /// Mark `version` [`KeyStatus::Retired`]. No-op if it isn't
    /// registered.
    pub fn retire(&mut self, version: u32) {
        self.set_status(version, KeyStatus::Retired);
    }

    fn active_entry(&self) -> Result<(&u32, &KeyRingEntry), Error> {
        self.entries
            .iter()
            .find(|(_, e)| e.status == KeyStatus::Active)
            .ok_or(Error::NoActiveKeyVersion)
    }

    /// Seal `plaintext` under the ring's current active key, stamping
    /// the resulting envelope with that key's version.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Envelope, Error> {
        let (version, entry) = self.active_entry()?;
        seal(&entry.key_pair.encapsulation_key, *version, plaintext)
    }

    /// Open `envelope` using whichever registered key matches its
    /// `key_version`, provided that key isn't [`KeyStatus::Retired`].
    pub fn open(&self, envelope: &Envelope) -> Result<Vec<u8>, Error> {
        let entry = self
            .entries
            .get(&envelope.key_version)
            .ok_or(Error::UnknownKeyVersion)?;
        if entry.status == KeyStatus::Retired {
            return Err(Error::RetiredKeyVersion);
        }
        open(&entry.key_pair.decapsulation_key, envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip() {
        let kp = KeyPair::generate();
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let envelope = seal(&kp.encapsulation_key, 1, plaintext).unwrap();
        let recovered = open(&kp.decapsulation_key, &envelope).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn envelope_byte_serialization_roundtrips() {
        let kp = KeyPair::generate();
        let envelope = seal(&kp.encapsulation_key, 1, b"payload").unwrap();
        let bytes = envelope.to_bytes();
        let parsed = Envelope::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, envelope);
        let recovered = open(&kp.decapsulation_key, &parsed).unwrap();
        assert_eq!(recovered, b"payload");
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let envelope = seal(&kp1.encapsulation_key, 1, b"secret").unwrap();
        assert_eq!(open(&kp2.decapsulation_key, &envelope), Err(Error::Decrypt));
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let kp = KeyPair::generate();
        let mut envelope = seal(&kp.encapsulation_key, 1, b"secret").unwrap();
        let last = envelope.aead_ciphertext.len() - 1;
        envelope.aead_ciphertext[last] ^= 0xFF;
        assert_eq!(open(&kp.decapsulation_key, &envelope), Err(Error::Decrypt));
    }

    #[test]
    fn malformed_bytes_are_rejected_not_panicking() {
        for len in 0..12 {
            let junk = vec![0xAAu8; len];
            assert!(Envelope::from_bytes(&junk).is_err());
        }
        // A kem_len prefix claiming more bytes than exist must fail
        // cleanly. Layout: [key_version: 4][kem_len: 4 = u32::MAX][10 more bytes].
        let mut bogus = 1u32.to_le_bytes().to_vec(); // key_version
        bogus.extend_from_slice(&u32::MAX.to_le_bytes()); // kem_len claims way more than exists
        bogus.extend_from_slice(&[0u8; 10]);
        assert_eq!(Envelope::from_bytes(&bogus), Err(Error::MalformedEnvelope));
    }

    #[test]
    fn same_seed_derives_the_same_keypair() {
        let seed = [0x42u8; SEED_LEN];
        let kp1 = KeyPair::from_seed(&seed);
        let kp2 = KeyPair::from_seed(&seed);

        // The two derivations must agree well enough that either one can
        // decrypt what the other sealed (the practical property replicas
        // actually need).
        let envelope = seal(&kp1.encapsulation_key, 1, b"shared across replicas").unwrap();
        let recovered = open(&kp2.decapsulation_key, &envelope).unwrap();
        assert_eq!(recovered, b"shared across replicas");
    }

    #[test]
    fn different_seeds_derive_unrelated_keypairs() {
        let kp1 = KeyPair::from_seed(&[0x11u8; SEED_LEN]);
        let kp2 = KeyPair::from_seed(&[0x22u8; SEED_LEN]);
        let envelope = seal(&kp1.encapsulation_key, 1, b"secret").unwrap();
        assert_eq!(open(&kp2.decapsulation_key, &envelope), Err(Error::Decrypt));
    }

    #[test]
    fn two_seals_of_the_same_plaintext_produce_different_ciphertexts() {
        // Fresh KEM encapsulation + fresh nonce each time -> no ciphertext reuse.
        let kp = KeyPair::generate();
        let e1 = seal(&kp.encapsulation_key, 1, b"same plaintext").unwrap();
        let e2 = seal(&kp.encapsulation_key, 1, b"same plaintext").unwrap();
        assert_ne!(e1.to_bytes(), e2.to_bytes());
    }

    #[test]
    fn envelope_serialization_roundtrips_key_version() {
        let kp = KeyPair::generate();
        let envelope = seal(&kp.encapsulation_key, 42, b"payload").unwrap();
        let bytes = envelope.to_bytes();
        let parsed = Envelope::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.key_version, 42);
    }

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
}
