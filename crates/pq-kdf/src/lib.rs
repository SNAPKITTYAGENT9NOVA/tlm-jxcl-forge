//! HKDF-SHA256 expansion of a KEM shared secret into an AES-256 key, with
//! fixed domain separation.
//!
//! # Scope
//!
//! This crate owns exactly the KDF step: turning an arbitrary-length
//! shared secret (typically a KEM shared secret from `pq-kem`) into a
//! fixed-length, purpose-bound AES-256 key. It knows nothing about KEMs
//! or AEAD -- see `pq-envelope` for the crate that composes this with
//! `pq-kem` and `pq-aead` into a full sealed value.
//!
//! Domain separation via [`HKDF_INFO`] means the same shared secret could
//! not accidentally be reused as a key for a different purpose even if it
//! were (mis)used elsewhere. Bumping the version suffix in that string is
//! a deliberate, documented way to invalidate all previously-derived keys
//! (and therefore all previously-sealed envelopes) if the construction
//! ever changes.
#![forbid(unsafe_code)]

use hkdf::Hkdf;
use sha2::Sha256;
use std::fmt;

/// Domain-separation string for the HKDF expand step. This must match
/// exactly between whoever derives a key and whoever later re-derives it
/// to decrypt something sealed under it -- see `pq-envelope`.
pub const HKDF_INFO: &[u8] = b"pq-crypto/aes-256-gcm-key/v1";

/// Length, in bytes, of the AES-256 key this crate derives.
pub const DERIVED_KEY_LEN: usize = 32;

/// Errors from the KDF step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// HKDF was asked to expand to an invalid output length. Unreachable
    /// in practice for the fixed 32-byte output this crate always
    /// requests (HKDF-SHA256 supports up to 255 * 32 = 8160 bytes), but
    /// modeled as a real, checked error rather than a panic or `unwrap`.
    KeyDerivation,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

/// Expand `shared_secret` via HKDF-SHA256 (no salt, i.e. the standard
/// zero-salt of hash-output length per RFC 5869) with this crate's fixed
/// [`HKDF_INFO`] domain-separation string, into a 32-byte AES-256 key.
pub fn derive_aes_key(shared_secret: &[u8]) -> Result<[u8; DERIVED_KEY_LEN], Error> {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut okm = [0u8; DERIVED_KEY_LEN];
    hk.expand(HKDF_INFO, &mut okm)
        .map_err(|_| Error::KeyDerivation)?;
    Ok(okm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_a_key_of_the_expected_length() {
        let key = derive_aes_key(b"some shared secret").unwrap();
        assert_eq!(key.len(), DERIVED_KEY_LEN);
    }

    #[test]
    fn same_input_derives_the_same_key() {
        let secret = b"a fixed 32-byte-ish shared secret";
        let k1 = derive_aes_key(secret).unwrap();
        let k2 = derive_aes_key(secret).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_inputs_derive_different_keys() {
        let k1 = derive_aes_key(b"shared secret one").unwrap();
        let k2 = derive_aes_key(b"shared secret two").unwrap();
        assert_ne!(k1, k2);
    }

    /// Known-answer test: HKDF-SHA256, no salt, `HKDF_INFO` as the info
    /// string, expanding a 32-byte input keying material of
    /// `0x00, 0x01, .., 0x1f` to 32 bytes of output key material. This
    /// exact value was independently computed via a from-scratch
    /// HMAC-SHA256-based HKDF-Extract/Expand implementation (RFC 5869),
    /// not derived from this crate's own code, and is pinned here as a
    /// regression vector: any future change to the KDF construction that
    /// silently changes derived key bytes for the same input must fail
    /// this test.
    #[test]
    fn known_answer_hkdf_sha256_vector() {
        let ikm: Vec<u8> = (0u8..32).collect();
        let key = derive_aes_key(&ikm).unwrap();
        let expected = [
            0xac, 0x91, 0x24, 0x16, 0xec, 0x41, 0x18, 0x7f, 0x7a, 0xe2, 0x42, 0xa1, 0x7d, 0x2c,
            0x97, 0x30, 0x62, 0xec, 0x81, 0xdc, 0xdf, 0xd5, 0x76, 0xea, 0x70, 0xf2, 0x46, 0x9d,
            0xd7, 0x06, 0xfe, 0x3d,
        ];
        assert_eq!(key, expected);
    }

    #[test]
    fn empty_shared_secret_is_still_accepted() {
        // HKDF-Extract accepts an empty IKM (it's still hashed with the
        // salt); this crate doesn't add an extra length precondition.
        assert!(derive_aes_key(&[]).is_ok());
    }
}
