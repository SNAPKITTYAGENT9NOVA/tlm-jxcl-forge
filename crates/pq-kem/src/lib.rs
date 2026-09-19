//! ML-KEM-768 (NIST FIPS 203, formerly CRYSTALS-Kyber) key generation and
//! encapsulation/decapsulation.
//!
//! # Scope
//!
//! This crate owns exactly the raw KEM step: generating a
//! ([`DecapsulationKey`], [`EncapsulationKey`]) pair (randomly or
//! deterministically from a seed) and the encapsulate/decapsulate
//! operations that establish a shared secret between them. It knows
//! nothing about what that shared secret is later used for -- turning it
//! into an AES key is `pq-kdf`'s job, and authenticated encryption under
//! that key is `pq-aead`'s job. See `pq-envelope` for the crate that
//! composes all three into a single sealed value.
//!
//! ML-KEM is believed secure against an attacker holding a
//! cryptographically relevant quantum computer, unlike classical
//! key-exchange schemes (RSA, ECDH) that Shor's algorithm breaks.
//!
//! [fips203]: https://csrc.nist.gov/pubs/fips/203/final
#![forbid(unsafe_code)]

use kem::{Decapsulate, Encapsulate};
use ml_kem::MlKem768;
use std::fmt;

/// This KEM's decapsulation (private) key type.
pub type DecapsulationKey = <MlKem768 as kem::Kem>::DecapsulationKey;
/// This KEM's encapsulation (public) key type.
pub type EncapsulationKey = <MlKem768 as kem::Kem>::EncapsulationKey;

/// Length in bytes of the seed accepted by [`KeyPair::from_seed`].
pub const SEED_LEN: usize = 64;

/// Length in bytes of the shared secret produced by [`encapsulate`] and
/// [`decapsulate`].
pub const SHARED_SECRET_LEN: usize = 32;

/// A key pair for this scheme. Generate one per service instance; there
/// is no need to persist it across restarts unless multiple replicas need
/// to share it (see [`KeyPair::from_seed`]).
pub struct KeyPair {
    pub decapsulation_key: DecapsulationKey,
    pub encapsulation_key: EncapsulationKey,
}

impl KeyPair {
    /// Generate a fresh key pair using the operating system's CSPRNG.
    ///
    /// Note: a randomly generated key pair is process-local. If multiple
    /// service replicas each call `generate()` independently, none of
    /// them can decrypt entries another replica sealed -- see
    /// [`KeyPair::from_seed`] for the fix when that matters.
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
    /// can decrypt each other's sealed values. The seed itself must be
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

/// Errors from the raw KEM step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The ciphertext handed to [`decapsulate`] was the wrong length for
    /// this KEM's parameter set.
    InvalidCiphertext,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

/// Encapsulate a fresh shared secret to the holder of `ek`, returning the
/// KEM ciphertext (to be sent to that holder) and the shared secret
/// (kept locally, e.g. to derive an encryption key from).
pub fn encapsulate(ek: &EncapsulationKey) -> (Vec<u8>, [u8; SHARED_SECRET_LEN]) {
    let (ciphertext, shared_secret) = ek.encapsulate();
    let mut secret = [0u8; SHARED_SECRET_LEN];
    secret.copy_from_slice(&shared_secret);
    (ciphertext.to_vec(), secret)
}

/// Decapsulate `ciphertext` with `dk`, recovering the shared secret that
/// [`encapsulate`] established for the matching [`EncapsulationKey`].
pub fn decapsulate(
    dk: &DecapsulationKey,
    ciphertext: &[u8],
) -> Result<[u8; SHARED_SECRET_LEN], Error> {
    let shared_secret = dk
        .decapsulate_slice(ciphertext)
        .map_err(|_| Error::InvalidCiphertext)?;
    let mut secret = [0u8; SHARED_SECRET_LEN];
    secret.copy_from_slice(&shared_secret);
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encapsulate_decapsulate_roundtrip_shares_a_secret() {
        let kp = KeyPair::generate();
        let (ciphertext, sent) = encapsulate(&kp.encapsulation_key);
        let received = decapsulate(&kp.decapsulation_key, &ciphertext).unwrap();
        assert_eq!(sent, received);
    }

    #[test]
    fn same_seed_derives_a_keypair_that_agrees_on_the_shared_secret() {
        // The practical property replicas actually need: two independent
        // `from_seed` calls with the same seed must produce keys that
        // can encapsulate/decapsulate for each other.
        let seed = [0x42u8; SEED_LEN];
        let kp1 = KeyPair::from_seed(&seed);
        let kp2 = KeyPair::from_seed(&seed);

        let (ciphertext, sent) = encapsulate(&kp1.encapsulation_key);
        let received = decapsulate(&kp2.decapsulation_key, &ciphertext).unwrap();
        assert_eq!(sent, received);
    }

    #[test]
    fn different_seeds_derive_unrelated_keypairs() {
        let kp1 = KeyPair::from_seed(&[0x11u8; SEED_LEN]);
        let kp2 = KeyPair::from_seed(&[0x22u8; SEED_LEN]);

        let (ciphertext, sent) = encapsulate(&kp1.encapsulation_key);
        let received = decapsulate(&kp2.decapsulation_key, &ciphertext).unwrap();
        assert_ne!(sent, received);
    }

    #[test]
    fn two_encapsulations_to_the_same_key_produce_different_ciphertexts_and_secrets() {
        let kp = KeyPair::generate();
        let (ct1, ss1) = encapsulate(&kp.encapsulation_key);
        let (ct2, ss2) = encapsulate(&kp.encapsulation_key);
        assert_ne!(ct1, ct2);
        assert_ne!(ss1, ss2);
    }

    #[test]
    fn truncated_ciphertext_is_rejected_not_panicking() {
        let kp = KeyPair::generate();
        let (ciphertext, _) = encapsulate(&kp.encapsulation_key);
        let truncated = &ciphertext[..ciphertext.len() - 1];
        assert_eq!(
            decapsulate(&kp.decapsulation_key, truncated),
            Err(Error::InvalidCiphertext)
        );
    }
}
