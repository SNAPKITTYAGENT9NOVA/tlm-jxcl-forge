//! AES-256-GCM authenticated encryption/decryption of the plaintext under
//! a derived key.
//!
//! # Scope
//!
//! This crate owns exactly the AEAD (authenticated encryption with
//! associated data -- though this construction uses no AAD) step: given a
//! 32-byte AES-256 key (produced by `pq-kdf`, though this crate doesn't
//! depend on it or know where the key came from) it encrypts/decrypts a
//! plaintext with a fresh random 96-bit nonce per encryption. See
//! `pq-envelope` for the crate that composes this with `pq-kem` and
//! `pq-kdf` into a full sealed value.
#![forbid(unsafe_code)]

use aes_gcm::{
    aead::{Aead, Generate, KeyInit, Nonce},
    Aes256Gcm, Key,
};
use std::fmt;

/// Length in bytes of the AES-256 key this crate expects.
pub const KEY_LEN: usize = 32;
/// Length in bytes of the GCM nonce this crate generates/expects.
pub const NONCE_LEN: usize = 12;

/// Errors from the AEAD step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// AEAD encryption failed. The `aes-gcm` crate deliberately gives no
    /// further detail, to avoid leaking side-channel information.
    Encrypt,
    /// AEAD decryption/authentication failed: wrong key, tampered
    /// ciphertext, or a foreign/corrupted ciphertext.
    Decrypt,
    /// The nonce handed to [`aead_decrypt`] was not exactly [`NONCE_LEN`]
    /// bytes.
    InvalidNonceLength,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

/// Encrypt `plaintext` under `key` with a freshly generated random nonce,
/// returning `(nonce, ciphertext)`. The nonce is not secret and must be
/// stored/transmitted alongside the ciphertext for decryption.
pub fn aead_encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*key));
    let nonce = Nonce::<Aes256Gcm>::generate();
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| Error::Encrypt)?;
    Ok((nonce.to_vec(), ciphertext))
}

/// Decrypt `ciphertext` under `key` and `nonce` (as produced by
/// [`aead_encrypt`]), verifying its authentication tag.
pub fn aead_decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, Error> {
    if nonce.len() != NONCE_LEN {
        return Err(Error::InvalidNonceLength);
    }
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*key));
    let nonce = Nonce::<Aes256Gcm>::try_from(nonce).map_err(|_| Error::InvalidNonceLength)?;
    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| Error::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = [0x11u8; KEY_LEN];
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let (nonce, ciphertext) = aead_encrypt(&key, plaintext).unwrap();
        let recovered = aead_decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let key1 = [0x11u8; KEY_LEN];
        let key2 = [0x22u8; KEY_LEN];
        let (nonce, ciphertext) = aead_encrypt(&key1, b"secret").unwrap();
        assert_eq!(
            aead_decrypt(&key2, &nonce, &ciphertext),
            Err(Error::Decrypt)
        );
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let key = [0x33u8; KEY_LEN];
        let (nonce, mut ciphertext) = aead_encrypt(&key, b"secret").unwrap();
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0xFF;
        assert_eq!(aead_decrypt(&key, &nonce, &ciphertext), Err(Error::Decrypt));
    }

    #[test]
    fn tampered_nonce_fails_to_decrypt() {
        let key = [0x44u8; KEY_LEN];
        let (mut nonce, ciphertext) = aead_encrypt(&key, b"secret").unwrap();
        nonce[0] ^= 0xFF;
        assert_eq!(aead_decrypt(&key, &nonce, &ciphertext), Err(Error::Decrypt));
    }

    #[test]
    fn wrong_length_nonce_is_rejected_not_panicking() {
        let key = [0x55u8; KEY_LEN];
        let (_, ciphertext) = aead_encrypt(&key, b"secret").unwrap();
        assert_eq!(
            aead_decrypt(&key, &[0u8; 4], &ciphertext),
            Err(Error::InvalidNonceLength)
        );
    }

    #[test]
    fn two_encryptions_of_the_same_plaintext_produce_different_nonces_and_ciphertexts() {
        let key = [0x66u8; KEY_LEN];
        let (nonce1, ct1) = aead_encrypt(&key, b"same plaintext").unwrap();
        let (nonce2, ct2) = aead_encrypt(&key, b"same plaintext").unwrap();
        assert_ne!(nonce1, nonce2);
        assert_ne!(ct1, ct2);
    }
}
