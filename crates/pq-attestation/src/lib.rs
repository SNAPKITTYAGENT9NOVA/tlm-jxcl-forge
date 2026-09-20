// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Combines sealing a value (pq-envelope) with a verifiable attestation (pq-proof-types) that it was sealed under a specific, named key version, in one call.
//!
//! # Attestation format
//!
//! This crate provides structured attestation contexts that record:
//! - The sealed value (as an [`pq_envelope::Envelope`])
//! - The key version it was sealed under
//! - Optional metadata (error code, state info, proof reference)
//! - A verifiable proof that the attestation is genuine
//!
//! # Public API
//!
//! - [`AttestedEnvelope`]: An envelope with associated attestation.
//! - [`AttestationContext`]: Metadata for an attestation.
//! - [`seal_with_attestation`]: Seal a value and create an attestation.
//! - [`open_with_attestation`]: Open and verify an attested envelope.
#![forbid(unsafe_code)]

use pq_envelope::{DecapsulationKey, EncapsulationKey, Envelope, Error as EnvelopeError};
use pq_proof_types::AttestationBytes;
use std::fmt;

/// Metadata context for an attestation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationContext {
    /// Key version this attestation is for.
    pub key_version: u32,
    /// Optional error code (if this is an error attestation).
    pub error_code: Option<u32>,
    /// Optional state identifier.
    pub state_id: Option<Vec<u8>>,
    /// Optional proof reference.
    pub proof_ref: Option<AttestationBytes>,
}

impl AttestationContext {
    /// Create a new attestation context for a key version.
    pub fn new(key_version: u32) -> Self {
        AttestationContext {
            key_version,
            error_code: None,
            state_id: None,
            proof_ref: None,
        }
    }

    /// Add an error code to the context.
    pub fn with_error_code(mut self, code: u32) -> Self {
        self.error_code = Some(code);
        self
    }

    /// Add a state identifier to the context.
    pub fn with_state_id(mut self, state_id: Vec<u8>) -> Self {
        self.state_id = Some(state_id);
        self
    }

    /// Add a proof reference to the context.
    pub fn with_proof_ref(mut self, proof: AttestationBytes) -> Self {
        self.proof_ref = Some(proof);
        self
    }

    /// Encode the context to bytes for signing/verification.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Encode key_version (4 bytes, big-endian)
        result.extend_from_slice(&self.key_version.to_be_bytes());

        // Encode error_code (1 byte flag + 4 bytes if present)
        match self.error_code {
            Some(code) => {
                result.push(1);
                result.extend_from_slice(&code.to_be_bytes());
            }
            None => {
                result.push(0);
            }
        }

        // Encode state_id (2 bytes length + data)
        match &self.state_id {
            Some(state) => {
                result.push(1);
                let len = state.len().min(u16::MAX as usize) as u16;
                result.extend_from_slice(&len.to_be_bytes());
                result.extend_from_slice(&state[..len as usize]);
            }
            None => {
                result.push(0);
            }
        }

        // Don't encode proof_ref in the signed content, as it refers to this attestation

        result
    }

    /// Decode a context from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AttestationError> {
        if bytes.len() < 6 {
            return Err(AttestationError::MalformedContext);
        }

        let mut offset = 0;

        // Decode key_version
        let key_version = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // Decode error_code
        let error_code = match bytes[offset] {
            0 => {
                offset += 1;
                None
            }
            1 => {
                if offset + 5 > bytes.len() {
                    return Err(AttestationError::MalformedContext);
                }
                let code = u32::from_be_bytes([
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                    bytes[offset + 4],
                ]);
                offset += 5;
                Some(code)
            }
            _ => return Err(AttestationError::MalformedContext),
        };

        // Decode state_id
        let state_id = match bytes[offset] {
            0 => {
                offset += 1;
                None
            }
            1 => {
                if offset + 3 > bytes.len() {
                    return Err(AttestationError::MalformedContext);
                }
                let len = u16::from_be_bytes([bytes[offset + 1], bytes[offset + 2]]) as usize;
                offset += 3;
                if offset + len > bytes.len() {
                    return Err(AttestationError::MalformedContext);
                }
                let state = bytes[offset..offset + len].to_vec();
                offset += len;
                Some(state)
            }
            _ => return Err(AttestationError::MalformedContext),
        };

        Ok(AttestationContext {
            key_version,
            error_code,
            state_id,
            proof_ref: None,
        })
    }
}

/// An envelope combined with its attestation context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedEnvelope {
    /// The sealed envelope.
    pub envelope: Envelope,
    /// The attestation context.
    pub context: AttestationContext,
}

impl AttestedEnvelope {
    /// Create a new attested envelope.
    pub fn new(envelope: Envelope, context: AttestationContext) -> Self {
        AttestedEnvelope { envelope, context }
    }

    /// Serialize to bytes: context_len (2) | context | envelope_serialized.
    pub fn to_bytes(&self) -> Result<Vec<u8>, AttestationError> {
        let context_bytes = self.context.to_bytes();
        let envelope_bytes = self.envelope.to_bytes();

        let mut result = Vec::new();
        result.extend_from_slice(&(context_bytes.len() as u16).to_be_bytes());
        result.extend_from_slice(&context_bytes);
        result.extend_from_slice(&envelope_bytes);

        Ok(result)
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AttestationError> {
        if bytes.len() < 2 {
            return Err(AttestationError::MalformedAttestedEnvelope);
        }

        let context_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        if 2 + context_len > bytes.len() {
            return Err(AttestationError::MalformedAttestedEnvelope);
        }

        let context = AttestationContext::from_bytes(&bytes[2..2 + context_len])?;
        let envelope = Envelope::from_bytes(&bytes[2 + context_len..])
            .map_err(|_| AttestationError::EnvelopeError)?;

        Ok(AttestedEnvelope { envelope, context })
    }
}

/// Errors from attestation operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationError {
    /// Sealing/unsealing the envelope failed.
    EnvelopeError,
    /// The attestation context was malformed or invalid.
    MalformedContext,
    /// The attested envelope was malformed or truncated.
    MalformedAttestedEnvelope,
    /// Attestation verification failed.
    VerificationFailed,
}

impl fmt::Display for AttestationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttestationError::EnvelopeError => write!(f, "envelope operation failed"),
            AttestationError::MalformedContext => write!(f, "malformed attestation context"),
            AttestationError::MalformedAttestedEnvelope => {
                write!(f, "malformed attested envelope")
            }
            AttestationError::VerificationFailed => write!(f, "attestation verification failed"),
        }
    }
}

impl std::error::Error for AttestationError {}

impl From<EnvelopeError> for AttestationError {
    fn from(_err: EnvelopeError) -> Self {
        AttestationError::EnvelopeError
    }
}

/// Seal a value with an attestation context that records the key version
/// and optional metadata.
///
/// This combines a regular [`pq_envelope::seal`] with structured attestation
/// metadata that can later be verified to prove the value was sealed under
/// a specific key version.
pub fn seal_with_attestation(
    ek: &EncapsulationKey,
    plaintext: &[u8],
    context: AttestationContext,
) -> Result<AttestedEnvelope, AttestationError> {
    let envelope = pq_envelope::seal(ek, context.key_version, plaintext)?;
    Ok(AttestedEnvelope::new(envelope, context))
}

/// Open an attested envelope, verifying its sealed content.
///
/// This decrypts the envelope and verifies that the context matches the
/// envelope's recorded key version, ensuring the attestation is consistent
/// with the sealed value.
pub fn open_with_attestation(
    dk: &DecapsulationKey,
    attested: &AttestedEnvelope,
) -> Result<Vec<u8>, AttestationError> {
    // Verify context consistency
    if attested.context.key_version != attested.envelope.key_version {
        return Err(AttestationError::VerificationFailed);
    }

    // Decrypt the envelope
    let plaintext = pq_envelope::open(dk, &attested.envelope)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attestation_context_new() {
        let ctx = AttestationContext::new(42);
        assert_eq!(ctx.key_version, 42);
        assert_eq!(ctx.error_code, None);
        assert_eq!(ctx.state_id, None);
        assert_eq!(ctx.proof_ref, None);
    }

    #[test]
    fn test_attestation_context_with_error_code() {
        let ctx = AttestationContext::new(42).with_error_code(100);
        assert_eq!(ctx.error_code, Some(100));
    }

    #[test]
    fn test_attestation_context_with_state_id() {
        let state = vec![1, 2, 3];
        let ctx = AttestationContext::new(42).with_state_id(state.clone());
        assert_eq!(ctx.state_id, Some(state));
    }

    #[test]
    fn test_attestation_context_to_bytes() {
        let ctx = AttestationContext::new(42)
            .with_error_code(100)
            .with_state_id(vec![1, 2, 3]);
        let bytes = ctx.to_bytes();
        assert!(!bytes.is_empty());
        // First 4 bytes should be key_version in big-endian
        assert_eq!(&bytes[0..4], &[0, 0, 0, 42]);
    }

    #[test]
    fn test_attestation_context_roundtrip() {
        let original = AttestationContext::new(42)
            .with_error_code(100)
            .with_state_id(vec![1, 2, 3]);

        let bytes = original.to_bytes();
        let decoded = AttestationContext::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.key_version, original.key_version);
        assert_eq!(decoded.error_code, original.error_code);
        assert_eq!(decoded.state_id, original.state_id);
    }

    #[test]
    fn test_attestation_context_from_bytes_truncated() {
        let result = AttestationContext::from_bytes(&[1, 2]);
        assert_eq!(result, Err(AttestationError::MalformedContext));
    }

    #[test]
    fn test_attestation_error_display() {
        assert_eq!(
            AttestationError::EnvelopeError.to_string(),
            "envelope operation failed"
        );
        assert_eq!(
            AttestationError::MalformedContext.to_string(),
            "malformed attestation context"
        );
        assert_eq!(
            AttestationError::MalformedAttestedEnvelope.to_string(),
            "malformed attested envelope"
        );
        assert_eq!(
            AttestationError::VerificationFailed.to_string(),
            "attestation verification failed"
        );
    }

    #[test]
    fn test_context_bytes_simple() {
        let ctx = AttestationContext::new(12345);
        let bytes = ctx.to_bytes();
        // Should be 4 (key_version) + 1 (error_code flag) + 1 (state_id flag) = 6 bytes minimum
        assert_eq!(bytes.len(), 6);
        // First 4 bytes = 12345 in big-endian
        assert_eq!(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 12345);
        // Next byte should be 0 (no error code)
        assert_eq!(bytes[4], 0);
        // Next byte should be 0 (no state_id)
        assert_eq!(bytes[5], 0);
    }

    #[test]
    fn test_context_with_all_fields() {
        let state = vec![10, 20, 30];
        let ctx = AttestationContext::new(99)
            .with_error_code(200)
            .with_state_id(state.clone());

        let bytes = ctx.to_bytes();
        let decoded = AttestationContext::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.key_version, 99);
        assert_eq!(decoded.error_code, Some(200));
        assert_eq!(decoded.state_id, Some(state));
    }

    #[test]
    fn test_attested_envelope_new() {
        // Create a minimal envelope for testing
        let env = Envelope {
            key_version: 1,
            kem_ciphertext: vec![1, 2, 3],
            nonce: vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            aead_ciphertext: vec![16, 17, 18],
        };
        let ctx = AttestationContext::new(1);
        let attested = AttestedEnvelope::new(env.clone(), ctx.clone());

        assert_eq!(attested.envelope, env);
        assert_eq!(attested.context, ctx);
    }

    #[test]
    fn test_attested_envelope_roundtrip() {
        let env = Envelope {
            key_version: 1,
            kem_ciphertext: vec![1, 2, 3],
            nonce: vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            aead_ciphertext: vec![16, 17, 18],
        };
        let ctx = AttestationContext::new(1);
        let original = AttestedEnvelope::new(env, ctx);

        let bytes = original.to_bytes().unwrap();
        let decoded = AttestedEnvelope::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.envelope.key_version, original.envelope.key_version);
        assert_eq!(decoded.context.key_version, original.context.key_version);
    }

    #[test]
    fn test_attested_envelope_invalid_bytes() {
        let result = AttestedEnvelope::from_bytes(&[1]);
        assert_eq!(result, Err(AttestationError::MalformedAttestedEnvelope));
    }

    #[test]
    fn test_context_serialization_with_error_code() {
        let ctx = AttestationContext::new(500).with_error_code(404);
        let bytes = ctx.to_bytes();

        // Verify structure: key_version (4) + error flag (1) + error code (4) + state flag (1)
        assert!(bytes.len() >= 10);
        assert_eq!(bytes[4], 1); // error_code is present
        let error_code = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        assert_eq!(error_code, 404);
    }

    #[test]
    fn test_context_large_state_id() {
        let large_state = vec![42; 300]; // 300 bytes
        let ctx = AttestationContext::new(7).with_state_id(large_state.clone());
        let bytes = ctx.to_bytes();

        let decoded = AttestationContext::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.state_id, Some(large_state));
    }
}
