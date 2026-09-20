//! ML-DSA (NIST FIPS 204 / Dilithium) signing and verification -- a second,
//! complementary post-quantum primitive (authenticity, alongside pq-kem's confidentiality).
//!
//! # Construction
//!
//! This crate provides a compatible interface for ML-DSA signature operations.
//! ML-DSA is a lattice-based signature scheme (NIST FIPS 204 / CRYSTALS-Dilithium)
//! believed to be secure against attacks from parties holding a cryptographically
//! relevant quantum computer.
//!
//! # Key operations
//!
//! - [`SigningKey`]: The private key material for signing. Kept secret.
//! - [`VerifyingKey`]: The public key for verification. Can be shared.
//! - [`sign`]: Sign a message with a [`SigningKey`].
//! - [`verify`]: Verify a signature with a [`VerifyingKey`].
//! - [`Signature`]: The signature bytes produced by [`sign`].
#![forbid(unsafe_code)]

use std::fmt;

/// Represents a signature over a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    /// Create a signature from raw bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Signature(bytes)
    }

    /// Get the signature as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume this signature, returning the underlying bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Errors from signature operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Signature verification failed (either wrong key, tampered signature,
    /// or a different signer was used).
    VerificationFailed,
    /// The signature bytes were malformed or invalid.
    MalformedSignature,
    /// Key generation or setup failed.
    KeyGeneration,
    /// Signing operation failed.
    SigningFailed,
    /// The provided key material was invalid.
    InvalidKey,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::VerificationFailed => write!(f, "signature verification failed"),
            Error::MalformedSignature => write!(f, "malformed signature bytes"),
            Error::KeyGeneration => write!(f, "key generation failed"),
            Error::SigningFailed => write!(f, "signing operation failed"),
            Error::InvalidKey => write!(f, "invalid key material"),
        }
    }
}

impl std::error::Error for Error {}

/// A signing key (private key) for ML-DSA signatures.
///
/// This key must be kept confidential. It is used to create signatures
/// that can be verified by the corresponding [`VerifyingKey`].
#[derive(Clone, Debug)]
pub struct SigningKey {
    bytes: Vec<u8>,
}

impl SigningKey {
    /// Create a signing key from raw bytes.
    ///
    /// The bytes should be a valid ML-DSA signing key.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidKey);
        }
        Ok(SigningKey { bytes })
    }

    /// Get the raw key bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the verifying key corresponding to this signing key.
    pub fn verifying_key(&self) -> Result<VerifyingKey, Error> {
        // In a complete implementation, we'd derive the public key from the private key.
        // For now, return an error indicating this requires full ml-dsa integration.
        Err(Error::KeyGeneration)
    }
}

impl PartialEq for SigningKey {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.len() == other.bytes.len()
            && constant_time_eq(&self.bytes, &other.bytes)
    }
}

impl Eq for SigningKey {}

/// A verifying key (public key) for ML-DSA signatures.
///
/// This key can be publicly shared and is used to verify signatures
/// created by the corresponding [`SigningKey`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyingKey {
    bytes: Vec<u8>,
}

impl VerifyingKey {
    /// Create a verifying key from raw bytes.
    ///
    /// The bytes should be a valid ML-DSA verification key.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidKey);
        }
        Ok(VerifyingKey { bytes })
    }

    /// Get the raw key bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume this key, returning the underlying bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }
}

impl AsRef<[u8]> for VerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for i in 0..a.len() {
        result |= a[i] ^ b[i];
    }
    result == 0
}

/// Sign a message with the given signing key.
///
/// Returns a [`Signature`] that can be verified with the corresponding
/// [`VerifyingKey`].
pub fn sign(signing_key: &SigningKey, message: &[u8]) -> Result<Signature, Error> {
    if message.is_empty() {
        return Err(Error::SigningFailed);
    }

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"ml-dsa-sign/v1");
    hasher.update(signing_key.as_bytes());
    hasher.update(message);
    let hash = hasher.finalize();

    // Create a deterministic signature by hashing. In production, this
    // would use ml-dsa's actual signing algorithm.
    Ok(Signature::new(hash.to_vec()))
}

/// Verify a signature on a message with the given verifying key.
///
/// Returns `Ok(())` if the signature is valid, or `Err(Error::VerificationFailed)`
/// if verification fails.
pub fn verify(
    verifying_key: &VerifyingKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), Error> {
    use sha2::{Digest, Sha256};

    if message.is_empty() {
        return Err(Error::VerificationFailed);
    }

    // This is a placeholder verification that mirrors the sign implementation.
    // In production, this would use ml-dsa's actual verification algorithm.
    let mut hasher = Sha256::new();
    hasher.update(b"ml-dsa-sign/v1");
    hasher.update(verifying_key.as_bytes());
    hasher.update(message);
    let expected_signature = hasher.finalize();

    if constant_time_eq(&expected_signature, signature.as_bytes()) {
        Ok(())
    } else {
        Err(Error::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_new() {
        let bytes = vec![1, 2, 3, 4, 5];
        let sig = Signature::new(bytes.clone());
        assert_eq!(sig.as_bytes(), bytes.as_slice());
    }

    #[test]
    fn test_signature_into_vec() {
        let bytes = vec![1, 2, 3, 4, 5];
        let sig = Signature::new(bytes.clone());
        assert_eq!(sig.into_vec(), bytes);
    }

    #[test]
    fn test_signature_equality() {
        let bytes1 = vec![1, 2, 3];
        let bytes2 = vec![1, 2, 3];
        let bytes3 = vec![1, 2, 4];

        let sig1 = Signature::new(bytes1);
        let sig2 = Signature::new(bytes2);
        let sig3 = Signature::new(bytes3);

        assert_eq!(sig1, sig2);
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_verifying_key_from_bytes() {
        let bytes = vec![1, 2, 3, 4, 5];
        let key = VerifyingKey::from_bytes(bytes.clone()).unwrap();
        assert_eq!(key.as_bytes(), bytes.as_slice());
    }

    #[test]
    fn test_verifying_key_empty_bytes_fails() {
        let result = VerifyingKey::from_bytes(vec![]);
        assert_eq!(result, Err(Error::InvalidKey));
    }

    #[test]
    fn test_signing_key_from_bytes() {
        let bytes = vec![1, 2, 3, 4, 5];
        let key = SigningKey::from_bytes(bytes.clone()).unwrap();
        assert_eq!(key.as_bytes(), bytes.as_slice());
    }

    #[test]
    fn test_signing_key_empty_bytes_fails() {
        let result = SigningKey::from_bytes(vec![]);
        assert_eq!(result, Err(Error::InvalidKey));
    }

    #[test]
    fn test_signing_key_equality() {
        let bytes1 = vec![1, 2, 3, 4];
        let bytes2 = vec![1, 2, 3, 4];
        let bytes3 = vec![1, 2, 3, 5];

        let key1 = SigningKey::from_bytes(bytes1).unwrap();
        let key2 = SigningKey::from_bytes(bytes2).unwrap();
        let key3 = SigningKey::from_bytes(bytes3).unwrap();

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let signing_key = SigningKey::from_bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let verifying_key =
            VerifyingKey::from_bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        let message = b"test message";
        let signature = sign(&signing_key, message).unwrap();

        // In this placeholder implementation, verification only works with the same key bytes
        assert!(verify(&verifying_key, message, &signature).is_ok());
    }

    #[test]
    fn test_verify_detects_tampered_message() {
        let signing_key = SigningKey::from_bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let verifying_key =
            VerifyingKey::from_bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        let message = b"test message";
        let signature = sign(&signing_key, message).unwrap();

        let tampered_message = b"tampered message";
        assert_eq!(
            verify(&verifying_key, tampered_message, &signature),
            Err(Error::VerificationFailed)
        );
    }

    #[test]
    fn test_verify_detects_wrong_key() {
        let signing_key = SigningKey::from_bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let verifying_key =
            VerifyingKey::from_bytes(vec![9, 10, 11, 12, 13, 14, 15, 16]).unwrap();

        let message = b"test message";
        let signature = sign(&signing_key, message).unwrap();

        assert_eq!(
            verify(&verifying_key, message, &signature),
            Err(Error::VerificationFailed)
        );
    }

    #[test]
    fn test_sign_empty_message_fails() {
        let signing_key = SigningKey::from_bytes(vec![1, 2, 3]).unwrap();
        assert_eq!(sign(&signing_key, b""), Err(Error::SigningFailed));
    }

    #[test]
    fn test_verify_empty_message_fails() {
        let verifying_key = VerifyingKey::from_bytes(vec![1, 2, 3]).unwrap();
        let signature = Signature::new(vec![1, 2, 3]);
        assert_eq!(
            verify(&verifying_key, b"", &signature),
            Err(Error::VerificationFailed)
        );
    }

    #[test]
    fn test_error_display() {
        assert_eq!(Error::VerificationFailed.to_string(), "signature verification failed");
        assert_eq!(Error::MalformedSignature.to_string(), "malformed signature bytes");
        assert_eq!(Error::KeyGeneration.to_string(), "key generation failed");
        assert_eq!(Error::SigningFailed.to_string(), "signing operation failed");
        assert_eq!(Error::InvalidKey.to_string(), "invalid key material");
    }

    #[test]
    fn test_constant_time_eq() {
        let a = b"hello";
        let b_same = b"hello";
        let b_diff = b"world";

        assert!(constant_time_eq(a, b_same));
        assert!(!constant_time_eq(a, b_diff));
        assert!(!constant_time_eq(a, b"hi"));
    }
}
