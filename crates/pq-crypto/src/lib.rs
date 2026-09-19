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

/// A sealed value: KEM ciphertext + AEAD nonce + AEAD ciphertext,
/// serializable to and from a single opaque byte blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
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
/// [`DecapsulationKey`] can recover it.
pub fn seal(ek: &EncapsulationKey, plaintext: &[u8]) -> Result<Envelope, Error> {
    let (kem_ciphertext, shared_secret) = ek.encapsulate();
    let key = derive_aes_key(&shared_secret)?;
    let cipher = Aes256Gcm::new(&key);

    let nonce = Nonce::<Aes256Gcm>::generate();
    let aead_ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| Error::Encrypt)?;

    Ok(Envelope {
        kem_ciphertext: kem_ciphertext.to_vec(),
        nonce: nonce.to_vec(),
        aead_ciphertext,
    })
}

/// Decrypt an [`Envelope`] previously produced by [`seal`] for the
/// matching [`EncapsulationKey`].
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
    /// `[kem_len: u32 LE][kem_ciphertext][nonce_len: u32 LE][nonce][aead_ciphertext]`.
    /// Length-prefixing (rather than hardcoding the KEM's fixed
    /// ciphertext size) keeps this format independent of which ML-KEM
    /// parameter set is in use.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            4 + self.kem_ciphertext.len() + 4 + self.nonce.len() + self.aead_ciphertext.len(),
        );
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

        let kem_len = read_u32(&mut pos, bytes)? as usize;
        let kem_ciphertext = take(&mut pos, kem_len, bytes)?;
        let nonce_len = read_u32(&mut pos, bytes)? as usize;
        let nonce = take(&mut pos, nonce_len, bytes)?;
        let aead_ciphertext = bytes.get(pos..).ok_or(Error::MalformedEnvelope)?.to_vec();

        Ok(Envelope {
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
        let envelope = seal(&kp.encapsulation_key, plaintext).unwrap();
        let recovered = open(&kp.decapsulation_key, &envelope).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn envelope_byte_serialization_roundtrips() {
        let kp = KeyPair::generate();
        let envelope = seal(&kp.encapsulation_key, b"payload").unwrap();
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
        let envelope = seal(&kp1.encapsulation_key, b"secret").unwrap();
        assert_eq!(open(&kp2.decapsulation_key, &envelope), Err(Error::Decrypt));
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let kp = KeyPair::generate();
        let mut envelope = seal(&kp.encapsulation_key, b"secret").unwrap();
        let last = envelope.aead_ciphertext.len() - 1;
        envelope.aead_ciphertext[last] ^= 0xFF;
        assert_eq!(open(&kp.decapsulation_key, &envelope), Err(Error::Decrypt));
    }

    #[test]
    fn malformed_bytes_are_rejected_not_panicking() {
        for len in 0..8 {
            let junk = vec![0xAAu8; len];
            assert!(Envelope::from_bytes(&junk).is_err());
        }
        // A length prefix claiming more bytes than exist must also fail cleanly.
        let mut bogus = (u32::MAX).to_le_bytes().to_vec();
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
        let envelope = seal(&kp1.encapsulation_key, b"shared across replicas").unwrap();
        let recovered = open(&kp2.decapsulation_key, &envelope).unwrap();
        assert_eq!(recovered, b"shared across replicas");
    }

    #[test]
    fn different_seeds_derive_unrelated_keypairs() {
        let kp1 = KeyPair::from_seed(&[0x11u8; SEED_LEN]);
        let kp2 = KeyPair::from_seed(&[0x22u8; SEED_LEN]);
        let envelope = seal(&kp1.encapsulation_key, b"secret").unwrap();
        assert_eq!(open(&kp2.decapsulation_key, &envelope), Err(Error::Decrypt));
    }

    #[test]
    fn two_seals_of_the_same_plaintext_produce_different_ciphertexts() {
        // Fresh KEM encapsulation + fresh nonce each time -> no ciphertext reuse.
        let kp = KeyPair::generate();
        let e1 = seal(&kp.encapsulation_key, b"same plaintext").unwrap();
        let e2 = seal(&kp.encapsulation_key, b"same plaintext").unwrap();
        assert_ne!(e1.to_bytes(), e2.to_bytes());
    }
}
