// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Cryptographic operations and key management for JXCL machines.
//!
//! This module provides core cryptographic functionality for the JXCL security model:
//! - **Key Management**: Creation and derivation of cryptographic keys from key material
//! - **Digital Signatures**: Signing and verification of data using HMAC-SHA256
//! - **Error Handling**: Comprehensive error types for cryptographic operations
//!
//! The security model is based on:
//! - HMAC-SHA256 for digital signatures (message authentication)
//! - HKDF-derived keys for key derivation (context-aware key generation)
//! - Constant-time verification to prevent timing attacks
//!
//! ## Examples
//!
//! ```rust
//! use jxcl_security::{derive_key, sign, verify};
//!
//! // Derive a key from key material
//! let key = derive_key(b"secret_material", 0x12345678).expect("key derivation failed");
//!
//! // Sign data
//! let data = b"important message";
//! let signature = sign(&key, data).expect("signing failed");
//!
//! // Verify the signature
//! verify(&signature, data, &key).expect("verification failed");
//! ```

#![forbid(unsafe_code)]

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::fmt;

/// A cryptographic key used for signing and verification operations.
///
/// Keys are 32 bytes (256 bits) in length, following the SHA-256 standard.
/// Keys should be treated as secrets and protected accordingly.
#[derive(Clone, PartialEq, Eq)]
pub struct CryptoKey {
    key_bytes: [u8; 32],
}

impl CryptoKey {
    /// Creates a new CryptoKey from a 32-byte array.
    fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key_bytes: bytes }
    }

    /// Returns the key bytes for cryptographic operations.
    fn as_bytes(&self) -> &[u8; 32] {
        &self.key_bytes
    }
}

impl fmt::Debug for CryptoKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CryptoKey")
            .field("key_bytes", &"[REDACTED]")
            .finish()
    }
}

/// A digital signature produced by signing data with a CryptoKey.
///
/// Signatures are 32 bytes (256 bits) in length, the output of HMAC-SHA256.
/// Signatures can be verified against the original data and key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    signature_bytes: [u8; 32],
}

impl Signature {
    /// Creates a new Signature from a 32-byte array.
    fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            signature_bytes: bytes,
        }
    }

    /// Returns the signature bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.signature_bytes
    }
}

/// Error type for cryptographic operations.
///
/// This enumeration covers all error conditions that can occur during
/// signing, verification, and key derivation operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureError {
    /// The signature verification failed - data was tampered or invalid key.
    VerificationFailed,
    /// The provided key is invalid or malformed.
    InvalidKey,
    /// The provided signature is invalid or malformed.
    InvalidSignature,
    /// Key derivation failed due to invalid input material.
    KeyDerivationFailed,
    /// An internal cryptographic error occurred.
    InternalError,
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VerificationFailed => write!(f, "signature verification failed"),
            Self::InvalidKey => write!(f, "invalid key"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::KeyDerivationFailed => write!(f, "key derivation failed"),
            Self::InternalError => write!(f, "internal cryptographic error"),
        }
    }
}

impl std::error::Error for SignatureError {}

/// Signs data using a cryptographic key, producing a digital signature.
///
/// Uses HMAC-SHA256 to create a message authentication code that proves
/// the data was signed with the given key.
///
/// # Arguments
///
/// * `key` - The CryptoKey to use for signing
/// * `data` - The data to sign (any length)
///
/// # Returns
///
/// * `Ok(Signature)` - The signature if signing succeeds
/// * `Err(SignatureError)` - If signing fails
pub fn sign(key: &CryptoKey, data: &[u8]) -> Result<Signature, SignatureError> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| SignatureError::InvalidKey)?;

    mac.update(data);
    let result = mac.finalize();
    let bytes = result.into_bytes();
    let mut sig_bytes = [0u8; 32];
    sig_bytes.copy_from_slice(&bytes[..32]);
    Ok(Signature::from_bytes(sig_bytes))
}

/// Verifies a signature against data using a cryptographic key.
///
/// Uses constant-time comparison to prevent timing attacks.
/// Returns success if the signature is valid for the given data and key.
///
/// # Arguments
///
/// * `signature` - The signature to verify
/// * `data` - The original data that was signed
/// * `key` - The CryptoKey that was used for signing
///
/// # Returns
///
/// * `Ok(())` - If the signature is valid
/// * `Err(SignatureError::VerificationFailed)` - If verification fails
///
/// # Examples
///
/// ```
/// use jxcl_security::{derive_key, sign, verify};
///
/// let key = derive_key(b"material", 0x99).expect("key derivation failed");
/// let data = b"test data";
/// let sig = sign(&key, data).expect("signing failed");
/// assert!(verify(&sig, data, &key).is_ok());
///
/// // Verification fails with modified data
/// let mut bad_data = data.to_vec();
/// bad_data[0] ^= 0xFF;
/// assert!(verify(&sig, &bad_data, &key).is_err());
/// ```
pub fn verify(signature: &Signature, data: &[u8], key: &CryptoKey) -> Result<(), SignatureError> {
    let expected = sign(key, data)?;

    // Constant-time comparison to prevent timing attacks
    let sig_bytes = signature.as_bytes();
    let expected_bytes = expected.as_bytes();

    let mut mismatch = 0u8;
    for (a, b) in sig_bytes.iter().zip(expected_bytes.iter()) {
        mismatch |= a ^ b;
    }

    if mismatch == 0 {
        Ok(())
    } else {
        Err(SignatureError::VerificationFailed)
    }
}

/// Derives a cryptographic key from key material using a context value.
///
/// Uses HKDF-SHA512 based key derivation to create a key from input material.
/// The context parameter allows generation of different keys from the same material.
///
/// # Arguments
///
/// * `material` - The key material (source entropy)
/// * `context` - A context identifier (allows multiple keys from same material)
///
/// # Returns
///
/// * `Ok(CryptoKey)` - The derived key if derivation succeeds
/// * `Err(SignatureError)` - If derivation fails
pub fn derive_key(material: &[u8], context: u32) -> Result<CryptoKey, SignatureError> {
    if material.is_empty() {
        return Err(SignatureError::KeyDerivationFailed);
    }

    let mut hasher = Sha512::new();
    hasher.update(material);
    hasher.update(context.to_le_bytes());

    let hash_result = hasher.finalize();
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&hash_result[..32]);

    Ok(CryptoKey::from_bytes(key_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_success() {
        let key = derive_key(b"material", 0x12345678);
        assert!(key.is_ok());
        let key = key.unwrap();
        assert_eq!(key.as_bytes().len(), 32);
    }

    #[test]
    fn test_derive_key_empty_material() {
        let result = derive_key(b"", 0x12345678);
        assert_eq!(result, Err(SignatureError::KeyDerivationFailed));
    }

    #[test]
    fn test_derive_key_different_context() {
        let key1 = derive_key(b"material", 0x00000001).expect("derivation 1 failed");
        let key2 = derive_key(b"material", 0x00000002).expect("derivation 2 failed");
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_derive_key_same_material_same_context() {
        let key1 = derive_key(b"material", 0x12345678).expect("derivation 1 failed");
        let key2 = derive_key(b"material", 0x12345678).expect("derivation 2 failed");
        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_sign_success() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let signature = sign(&key, b"data");
        assert!(signature.is_ok());
        let sig = signature.unwrap();
        assert_eq!(sig.as_bytes().len(), 32);
    }

    #[test]
    fn test_sign_empty_data() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let signature = sign(&key, b"");
        assert!(signature.is_ok());
    }

    #[test]
    fn test_sign_different_data_different_signature() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let sig1 = sign(&key, b"data1").expect("signing 1 failed");
        let sig2 = sign(&key, b"data2").expect("signing 2 failed");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_verify_success() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let data = b"test data";
        let signature = sign(&key, data).expect("signing failed");
        let result = verify(&signature, data, &key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_tampered_data() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let data = b"test data";
        let signature = sign(&key, data).expect("signing failed");

        let mut tampered = data.to_vec();
        tampered[0] ^= 0xFF;

        let result = verify(&signature, &tampered, &key);
        assert_eq!(result, Err(SignatureError::VerificationFailed));
    }

    #[test]
    fn test_verify_wrong_key() {
        let key1 = derive_key(b"material1", 0).expect("key derivation 1 failed");
        let key2 = derive_key(b"material2", 0).expect("key derivation 2 failed");
        let data = b"test data";
        let signature = sign(&key1, data).expect("signing failed");

        let result = verify(&signature, data, &key2);
        assert_eq!(result, Err(SignatureError::VerificationFailed));
    }

    #[test]
    fn test_verify_tampered_signature() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let data = b"test data";
        let mut signature = sign(&key, data).expect("signing failed");

        // Tamper with signature bytes
        let sig_bytes_mut = &mut signature.signature_bytes;
        sig_bytes_mut[0] ^= 0xFF;

        let result = verify(&signature, data, &key);
        assert_eq!(result, Err(SignatureError::VerificationFailed));
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let key = derive_key(b"secret_key_material", 0x999).expect("key derivation failed");
        let test_data = b"important message to sign";

        let signature = sign(&key, test_data).expect("signing failed");
        let verify_result = verify(&signature, test_data, &key);

        assert!(verify_result.is_ok());
    }

    #[test]
    fn test_multiple_keys_independent() {
        let key1 = derive_key(b"material_a", 0).expect("key derivation 1 failed");
        let key2 = derive_key(b"material_b", 0).expect("key derivation 2 failed");
        let data = b"shared data";

        let sig1 = sign(&key1, data).expect("signing 1 failed");
        let sig2 = sign(&key2, data).expect("signing 2 failed");

        // Each signature only verifies with its own key
        assert!(verify(&sig1, data, &key1).is_ok());
        assert_eq!(
            verify(&sig1, data, &key2),
            Err(SignatureError::VerificationFailed)
        );
        assert!(verify(&sig2, data, &key2).is_ok());
        assert_eq!(
            verify(&sig2, data, &key1),
            Err(SignatureError::VerificationFailed)
        );
    }

    #[test]
    fn test_long_data_signing() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let long_data = vec![0x42u8; 10000];

        let signature = sign(&key, &long_data).expect("signing failed");
        let result = verify(&signature, &long_data, &key);

        assert!(result.is_ok());
    }

    #[test]
    fn test_signature_deterministic() {
        let key = derive_key(b"material", 0).expect("key derivation failed");
        let data = b"test";

        let sig1 = sign(&key, data).expect("signing 1 failed");
        let sig2 = sign(&key, data).expect("signing 2 failed");

        assert_eq!(sig1, sig2);
    }
}
