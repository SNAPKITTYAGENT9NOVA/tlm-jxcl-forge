// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The sealed-value wire format: key version, KEM ciphertext, nonce, AEAD
//! ciphertext -- and the `seal`/`open` orchestration that produces and
//! consumes it.
//!
//! # Construction
//!
//! This is a standard hybrid public-key encryption (KEM/DEM) scheme,
//! composed from three lower-level crates:
//!
//! 1. **KEM** ([`pq_kem`]): [ML-KEM-768][fips203] (NIST FIPS 203, formerly
//!    CRYSTALS-Kyber) encapsulates a fresh 32-byte shared secret to the
//!    holder of a [`DecapsulationKey`], producing a KEM ciphertext.
//!    ML-KEM is believed secure against an attacker holding a
//!    cryptographically relevant quantum computer, unlike classical
//!    key-exchange schemes (RSA, ECDH) that Shor's algorithm breaks.
//! 2. **KDF** ([`pq_kdf`]): the shared secret is expanded via HKDF-SHA256
//!    with a fixed, versioned info string, into a 256-bit AES key.
//! 3. **DEM** ([`pq_aead`]): the plaintext is encrypted with AES-256-GCM
//!    under that derived key and a fresh random 96-bit nonce.
//!
//! [`Envelope`] bundles the KEM ciphertext, nonce, and AEAD ciphertext
//! together into one self-describing byte blob suitable for storing as an
//! opaque value in a cache or database (see the `pq-cache`/`pq-sql-vault`
//! crates).
//!
//! # Threat model
//!
//! This protects the *confidentiality of a sealed value at rest* against
//! anyone who can read the storage it ends up in but does not hold the
//! [`DecapsulationKey`] -- including an attacker with a quantum computer.
//! It does **not** by itself secure the network transport to that
//! storage; that is a separate, orthogonal concern (e.g. TLS/mTLS).
//!
//! # This crate's [`Error`]
//!
//! [`Error`] is the single error currency for this whole subsystem: in
//! addition to the six variants this crate's own [`seal`]/[`open`]
//! produce, it also carries the three variants `pq-keyring`/`pq-rotation`
//! need for ring-level lookups (`UnknownKeyVersion`, `RetiredKeyVersion`,
//! `NoActiveKeyVersion`). Those crates depend on this one (directly or
//! transitively) and construct those variants themselves -- a public enum
//! with public variants can be constructed by any downstream crate, so
//! this needs no upward dependency from here. Keeping one flat `Error`
//! type across the whole KEM/KDF/AEAD/ring stack (rather than a different
//! error type per crate that callers would have to convert between) is
//! the same "one error currency" choice `jxcl-errors` makes for its own
//! generic `Error` enum.
//!
//! [fips203]: https://csrc.nist.gov/pubs/fips/203/final
#![forbid(unsafe_code)]

use std::fmt;

pub use pq_kem::{DecapsulationKey, EncapsulationKey, KeyPair, SEED_LEN};

const NONCE_LEN: usize = 12;

/// A sealed value: which key version sealed it, plus the KEM ciphertext,
/// AEAD nonce, and AEAD ciphertext, serializable to and from a single
/// opaque byte blob.
///
/// `key_version` is what makes key rotation possible: a `KeyRing`
/// (`pq-keyring`/`pq-rotation`) stamps every envelope with the version
/// that sealed it, so a later `open()` against a *different* active
/// version can still find and use the right (now decrypt-only) key for
/// entries sealed before the rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub key_version: u32,
    pub kem_ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub aead_ciphertext: Vec<u8>,
}

/// Errors from sealing/opening a value, or from a [`KeyRing`]-style
/// lookup built on top of this crate (see the module-level "This crate's
/// `Error`" section for why the latter three variants live here).
///
/// [`KeyRing`]: https://docs.rs/pq-rotation (conceptually; this crate
/// doesn't depend on it)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// HKDF was asked to expand to an invalid output length (should be
    /// unreachable for the fixed 32-byte AES-256 key this crate derives).
    KeyDerivation,
    /// AEAD encryption failed (the underlying AEAD crate deliberately
    /// gives no further detail, to avoid leaking side-channel
    /// information).
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
    /// A key ring has no key registered for the envelope's declared
    /// `key_version`.
    UnknownKeyVersion,
    /// The key ring's key for this envelope's `key_version` has been
    /// retired and is no longer available for decryption.
    RetiredKeyVersion,
    /// A key ring was asked to seal a new value but has no active key
    /// version to seal it under.
    NoActiveKeyVersion,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

/// Encrypt `plaintext` so that only the holder of the matching
/// [`DecapsulationKey`] can recover it, stamping the envelope with
/// `key_version` (see `pq-rotation`'s `KeyRing` for the usual,
/// rotation-aware way to call this rather than tracking versions by
/// hand).
pub fn seal(ek: &EncapsulationKey, key_version: u32, plaintext: &[u8]) -> Result<Envelope, Error> {
    let (kem_ciphertext, shared_secret) = pq_kem::encapsulate(ek);
    let key = pq_kdf::derive_aes_key(&shared_secret).map_err(|_| Error::KeyDerivation)?;
    let (nonce, aead_ciphertext) =
        pq_aead::aead_encrypt(&key, plaintext).map_err(|_| Error::Encrypt)?;

    Ok(Envelope {
        key_version,
        kem_ciphertext,
        nonce,
        aead_ciphertext,
    })
}

/// Decrypt an [`Envelope`] previously produced by [`seal`] for the
/// matching [`EncapsulationKey`]. Callers that manage multiple key
/// versions should generally use `pq-rotation`'s `KeyRing::open` instead,
/// which picks the right `dk` for the envelope's `key_version`
/// automatically.
pub fn open(dk: &DecapsulationKey, envelope: &Envelope) -> Result<Vec<u8>, Error> {
    if envelope.nonce.len() != NONCE_LEN {
        return Err(Error::InvalidNonceLength);
    }
    let shared_secret = pq_kem::decapsulate(dk, &envelope.kem_ciphertext)
        .map_err(|_| Error::InvalidKemCiphertext)?;
    let key = pq_kdf::derive_aes_key(&shared_secret).map_err(|_| Error::KeyDerivation)?;
    pq_aead::aead_decrypt(&key, &envelope.nonce, &envelope.aead_ciphertext)
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
    fn same_seed_derives_a_keypair_that_can_open_what_the_other_sealed() {
        // The practical property replicas actually need: two independent
        // `from_seed` calls with the same seed must be interchangeable.
        let seed = [0x42u8; SEED_LEN];
        let kp1 = KeyPair::from_seed(&seed);
        let kp2 = KeyPair::from_seed(&seed);
        let envelope = seal(&kp1.encapsulation_key, 1, b"shared across replicas").unwrap();
        let recovered = open(&kp2.decapsulation_key, &envelope).unwrap();
        assert_eq!(recovered, b"shared across replicas");
    }
}
