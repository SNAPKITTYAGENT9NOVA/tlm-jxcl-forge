// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A tamper-evident, hash-chained audit ledger recording proof attestations in order, built on pq-journal.
//!
//! # Overview
//!
//! `pq-ledger` provides an immutable, append-only ledger for audit trails and proof attestations.
//! Each entry is cryptographically linked to the previous one via hash-chaining, providing
//! tamper-evidence. Ledger entries can be signed for authenticity.
//!
//! # Features
//!
//! - **Hash-chained entries**: Each entry includes a hash of the previous entry
//! - **Tamper-evident**: Any modification to a past entry changes its hash, breaking the chain
//! - **Signed commits**: Entries can be signed with a cryptographic key
//! - **Verification**: Full chain verification ensures integrity from genesis to any entry
//! - **Built on pq-journal**: Uses the write-ahead journal for durability
//!
//! # Entry Format
//!
//! Each ledger entry contains:
//! - Previous hash (32 bytes, SHA256): Hash of the previous entry's serialized form
//! - Entry sequence number (8 bytes, u64): Incrementing counter for ordering
//! - Entry timestamp (8 bytes, u64): Nanoseconds since epoch (or similar)
//! - Attestation data (variable): The actual proof attestation bytes
//! - Optional signature (variable): Signature over the entry, if signed
//!
//! Owns: Ledger::record/verify_chain.
#![forbid(unsafe_code)]

use jxcl_bytes::{ByteCursor, ByteCursorMut};
use pq_journal::{Journal, JournalEntry, JournalError};
use pq_proof_types::AttestationBytes;
use sha2::{Digest, Sha256};
use std::fmt;

/// Error type for ledger operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// Journal operation failed.
    Journal(String),
    /// Entry is corrupted or invalid.
    Corrupted(String),
    /// Chain verification failed (entry hash doesn't match expected hash).
    VerificationFailed(String),
    /// Invalid signature on an entry.
    InvalidSignature(String),
    /// IO or serialization error.
    Io(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::Journal(msg) => write!(f, "journal error: {}", msg),
            LedgerError::Corrupted(msg) => write!(f, "corrupted entry: {}", msg),
            LedgerError::VerificationFailed(msg) => write!(f, "verification failed: {}", msg),
            LedgerError::InvalidSignature(msg) => write!(f, "invalid signature: {}", msg),
            LedgerError::Io(msg) => write!(f, "io error: {}", msg),
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<JournalError> for LedgerError {
    fn from(e: JournalError) -> Self {
        LedgerError::Journal(e.to_string())
    }
}

/// Hash type for ledger entries (SHA256 = 32 bytes).
pub type Hash = [u8; 32];

/// The zero/genesis hash (all zeros).
fn genesis_hash() -> Hash {
    [0u8; 32]
}

/// Compute SHA256 hash of the given bytes.
fn sha256(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    hash
}

/// A single entry in the ledger, containing proof attestation data.
///
/// Each entry is cryptographically linked to the previous one via its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Hash of the previous entry's serialized form (genesis = all zeros).
    previous_hash: Hash,
    /// Sequence number of this entry.
    sequence: u64,
    /// Timestamp (nanoseconds since epoch, or application-defined).
    timestamp: u64,
    /// The attestation data (opaque, application-specific).
    attestation: AttestationBytes,
    /// Optional signature over this entry's serialized form.
    signature: Option<Vec<u8>>,
}

impl Entry {
    /// Create a new ledger entry.
    pub fn new(
        previous_hash: Hash,
        sequence: u64,
        timestamp: u64,
        attestation: AttestationBytes,
    ) -> Self {
        Entry {
            previous_hash,
            sequence,
            timestamp,
            attestation,
            signature: None,
        }
    }

    /// Create an entry with a signature.
    pub fn with_signature(
        previous_hash: Hash,
        sequence: u64,
        timestamp: u64,
        attestation: AttestationBytes,
        signature: Vec<u8>,
    ) -> Self {
        Entry {
            previous_hash,
            sequence,
            timestamp,
            attestation,
            signature: Some(signature),
        }
    }

    /// Get the previous entry's hash.
    pub fn previous_hash(&self) -> &Hash {
        &self.previous_hash
    }

    /// Get the sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get the timestamp.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Get the attestation data.
    pub fn attestation(&self) -> &AttestationBytes {
        &self.attestation
    }

    /// Get the signature, if present.
    pub fn signature(&self) -> Option<&[u8]> {
        self.signature.as_deref()
    }

    /// Check if this entry has a signature.
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Compute the hash of this entry.
    ///
    /// The hash is computed over the serialized form of the entry
    /// (without the signature, if present).
    pub fn hash(&self) -> Hash {
        let serialized = self.serialize_without_signature().unwrap_or_default();
        sha256(&serialized)
    }

    /// Serialize this entry to bytes without signature (for hashing and signing).
    fn serialize_without_signature(&self) -> Result<Vec<u8>, LedgerError> {
        let mut cursor = ByteCursorMut::new();

        // Serialize: previous_hash (32) + sequence (8) + timestamp (8) + attestation_len (4) + attestation
        cursor
            .write_bytes(&self.previous_hash)
            .map_err(|e| LedgerError::Io(format!("failed to write previous_hash: {}", e)))?;
        cursor
            .write_u64(self.sequence)
            .map_err(|e| LedgerError::Io(format!("failed to write sequence: {}", e)))?;
        cursor
            .write_u64(self.timestamp)
            .map_err(|e| LedgerError::Io(format!("failed to write timestamp: {}", e)))?;
        cursor
            .write_u32(self.attestation.as_slice().len() as u32)
            .map_err(|e| LedgerError::Io(format!("failed to write attestation length: {}", e)))?;
        cursor
            .write_bytes(self.attestation.as_slice())
            .map_err(|e| LedgerError::Io(format!("failed to write attestation: {}", e)))?;

        Ok(cursor.into_vec())
    }

    /// Serialize this entry to bytes including signature.
    fn to_bytes(&self) -> Result<Vec<u8>, LedgerError> {
        let mut cursor = ByteCursorMut::new();

        // Serialize the unsigned part
        cursor
            .write_bytes(&self.serialize_without_signature()?)
            .map_err(|e| LedgerError::Io(format!("failed to write entry data: {}", e)))?;

        // Serialize signature presence and content
        if let Some(sig) = &self.signature {
            cursor
                .write_u8(1)
                .map_err(|e| LedgerError::Io(format!("failed to write signature flag: {}", e)))?;
            cursor
                .write_u32(sig.len() as u32)
                .map_err(|e| LedgerError::Io(format!("failed to write signature length: {}", e)))?;
            cursor
                .write_bytes(sig)
                .map_err(|e| LedgerError::Io(format!("failed to write signature: {}", e)))?;
        } else {
            cursor
                .write_u8(0)
                .map_err(|e| LedgerError::Io(format!("failed to write signature flag: {}", e)))?;
        }

        Ok(cursor.into_vec())
    }

    /// Deserialize an entry from bytes.
    fn from_bytes(data: &[u8]) -> Result<Entry, LedgerError> {
        let mut cursor = ByteCursor::new(data);

        // Read previous hash
        let mut previous_hash = [0u8; 32];
        let hash_bytes = cursor
            .read_bytes(32)
            .map_err(|e| LedgerError::Io(format!("failed to read previous_hash: {}", e)))?;
        previous_hash.copy_from_slice(hash_bytes);

        // Read sequence
        let sequence = cursor
            .read_u64()
            .map_err(|e| LedgerError::Io(format!("failed to read sequence: {}", e)))?;

        // Read timestamp
        let timestamp = cursor
            .read_u64()
            .map_err(|e| LedgerError::Io(format!("failed to read timestamp: {}", e)))?;

        // Read attestation
        let attestation_len = cursor
            .read_u32()
            .map_err(|e| LedgerError::Io(format!("failed to read attestation length: {}", e)))?
            as usize;
        let attestation_bytes = cursor
            .read_bytes(attestation_len)
            .map_err(|e| LedgerError::Io(format!("failed to read attestation: {}", e)))?;
        let attestation = AttestationBytes::new(attestation_bytes.to_vec());

        // Read signature
        let has_signature = cursor
            .read_u8()
            .map_err(|e| LedgerError::Io(format!("failed to read signature flag: {}", e)))?;
        let signature = if has_signature != 0 {
            let sig_len = cursor
                .read_u32()
                .map_err(|e| LedgerError::Io(format!("failed to read signature length: {}", e)))?
                as usize;
            let sig_bytes = cursor
                .read_bytes(sig_len)
                .map_err(|e| LedgerError::Io(format!("failed to read signature: {}", e)))?;
            Some(sig_bytes.to_vec())
        } else {
            None
        };

        Ok(Entry {
            previous_hash,
            sequence,
            timestamp,
            attestation,
            signature,
        })
    }
}

/// An immutable, append-only audit ledger built on pq-journal.
///
/// The ledger maintains a hash-chained sequence of entries, each cryptographically
/// linked to its predecessor. This provides tamper-evident storage where any
/// modification to a past entry can be detected.
#[derive(Debug)]
pub struct Ledger {
    journal: Journal,
    /// Cache of computed entry hashes, indexed by sequence number.
    hash_cache: std::collections::BTreeMap<u64, Hash>,
    /// The sequence number of the next entry to be added.
    next_sequence: u64,
}

impl Ledger {
    /// Create a new, empty ledger.
    pub fn new() -> Self {
        Ledger {
            journal: Journal::new(),
            hash_cache: std::collections::BTreeMap::new(),
            next_sequence: 0,
        }
    }

    /// Create a ledger from persisted journal data.
    pub fn from_bytes(journal_data: Vec<u8>) -> Result<Self, LedgerError> {
        let journal = Journal::from_bytes(journal_data)?;
        let mut ledger = Ledger {
            journal,
            hash_cache: std::collections::BTreeMap::new(),
            next_sequence: 0,
        };

        // Scan entries to populate cache and verify
        ledger.verify_chain()?;

        Ok(ledger)
    }

    /// Append a new entry to the ledger.
    ///
    /// The entry's previous_hash should be the hash of the last entry
    /// (or genesis hash for the first entry).
    pub fn append_entry(&mut self, entry: Entry) -> Result<(), LedgerError> {
        // Validate that this entry's sequence matches what we expect
        if entry.sequence != self.next_sequence {
            return Err(LedgerError::Corrupted(format!(
                "entry sequence {} doesn't match expected {}",
                entry.sequence, self.next_sequence
            )));
        }

        // Validate chain continuity
        let expected_previous_hash = if self.next_sequence == 0 {
            genesis_hash()
        } else {
            self.hash_cache
                .get(&(self.next_sequence - 1))
                .copied()
                .ok_or_else(|| {
                    LedgerError::Corrupted("previous entry hash not cached".to_string())
                })?
        };

        if entry.previous_hash != expected_previous_hash {
            return Err(LedgerError::VerificationFailed(
                "entry's previous_hash doesn't match expected hash".to_string(),
            ));
        }

        // Compute and cache this entry's hash
        let entry_hash = entry.hash();
        self.hash_cache.insert(self.next_sequence, entry_hash);

        // Serialize and append to journal
        let serialized = entry.to_bytes()?;
        let journal_entry = JournalEntry::new(serialized);
        self.journal.append(journal_entry)?;

        self.next_sequence += 1;
        Ok(())
    }

    /// Append a signed entry (convenience method).
    pub fn signed_commit(
        &mut self,
        sequence: u64,
        timestamp: u64,
        attestation: AttestationBytes,
        signature: Vec<u8>,
    ) -> Result<(), LedgerError> {
        let previous_hash = if sequence == 0 {
            genesis_hash()
        } else {
            self.hash_cache
                .get(&(sequence - 1))
                .copied()
                .ok_or_else(|| {
                    LedgerError::Corrupted("previous entry hash not cached".to_string())
                })?
        };

        let entry =
            Entry::with_signature(previous_hash, sequence, timestamp, attestation, signature);
        self.append_entry(entry)
    }

    /// Verify the entire chain from genesis to the latest entry.
    ///
    /// Returns the number of entries in the ledger.
    pub fn verify_chain(&mut self) -> Result<usize, LedgerError> {
        let entries = self.journal.scan()?;
        self.hash_cache.clear();
        self.next_sequence = 0;

        let mut expected_previous_hash = genesis_hash();

        for serialized_entry in entries {
            let entry = Entry::from_bytes(serialized_entry.as_slice()).map_err(|e| {
                LedgerError::Corrupted(format!("failed to deserialize entry: {}", e))
            })?;

            // Verify chain continuity
            if entry.previous_hash != expected_previous_hash {
                return Err(LedgerError::VerificationFailed(format!(
                    "entry {} has incorrect previous_hash",
                    self.next_sequence
                )));
            }

            // Verify sequence
            if entry.sequence != self.next_sequence {
                return Err(LedgerError::VerificationFailed(format!(
                    "entry has sequence {}, expected {}",
                    entry.sequence, self.next_sequence
                )));
            }

            // Compute and cache hash
            let entry_hash = entry.hash();
            self.hash_cache.insert(self.next_sequence, entry_hash);

            expected_previous_hash = entry_hash;
            self.next_sequence += 1;
        }

        Ok(self.next_sequence as usize)
    }

    /// Get the current tip hash (hash of the last entry), or genesis hash if empty.
    pub fn tip_hash(&self) -> Hash {
        if self.next_sequence == 0 {
            genesis_hash()
        } else {
            self.hash_cache
                .get(&(self.next_sequence - 1))
                .copied()
                .unwrap_or(genesis_hash())
        }
    }

    /// Get all entries in the ledger.
    pub fn entries(&self) -> Result<Vec<Entry>, LedgerError> {
        let journal_entries = self.journal.scan()?;
        journal_entries
            .iter()
            .map(|je| Entry::from_bytes(je.as_slice()))
            .collect()
    }

    /// Get the number of entries in the ledger.
    pub fn len(&self) -> usize {
        self.next_sequence as usize
    }

    /// Check if the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.next_sequence == 0
    }

    /// Get the raw journal bytes for persistence.
    pub fn as_bytes(&self) -> &[u8] {
        self.journal.as_bytes()
    }

    /// Consume the ledger and return its journal bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.journal.into_bytes()
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_create() {
        let attestation = AttestationBytes::new(vec![1, 2, 3]);
        let entry = Entry::new(genesis_hash(), 0, 1000, attestation.clone());

        assert_eq!(entry.sequence(), 0);
        assert_eq!(entry.timestamp(), 1000);
        assert_eq!(entry.attestation(), &attestation);
        assert!(!entry.is_signed());
    }

    #[test]
    fn test_entry_with_signature() {
        let attestation = AttestationBytes::new(vec![1, 2, 3]);
        let sig = vec![4, 5, 6];
        let entry = Entry::with_signature(genesis_hash(), 0, 1000, attestation, sig.clone());

        assert!(entry.is_signed());
        assert_eq!(entry.signature(), Some(&sig[..]));
    }

    #[test]
    fn test_entry_hash_deterministic() {
        let attestation = AttestationBytes::new(vec![1, 2, 3]);
        let entry1 = Entry::new(genesis_hash(), 0, 1000, attestation.clone());
        let entry2 = Entry::new(genesis_hash(), 0, 1000, attestation);

        assert_eq!(entry1.hash(), entry2.hash());
    }

    #[test]
    fn test_entry_hash_changes_with_data() {
        let entry1 = Entry::new(
            genesis_hash(),
            0,
            1000,
            AttestationBytes::new(vec![1, 2, 3]),
        );
        let entry2 = Entry::new(
            genesis_hash(),
            0,
            1000,
            AttestationBytes::new(vec![1, 2, 4]),
        );

        assert_ne!(entry1.hash(), entry2.hash());
    }

    #[test]
    fn test_entry_serialization_round_trip() {
        let attestation = AttestationBytes::new(vec![1, 2, 3]);
        let entry = Entry::new(genesis_hash(), 0, 1000, attestation.clone());

        let bytes = entry.to_bytes().unwrap();
        let deserialized = Entry::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.sequence(), 0);
        assert_eq!(deserialized.timestamp(), 1000);
        assert_eq!(deserialized.attestation(), &attestation);
    }

    #[test]
    fn test_entry_with_signature_serialization() {
        let attestation = AttestationBytes::new(vec![1, 2, 3]);
        let sig = vec![4, 5, 6];
        let entry =
            Entry::with_signature(genesis_hash(), 0, 1000, attestation.clone(), sig.clone());

        let bytes = entry.to_bytes().unwrap();
        let deserialized = Entry::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.signature(), Some(&sig[..]));
    }

    #[test]
    fn test_ledger_create() {
        let ledger = Ledger::new();
        assert!(ledger.is_empty());
        assert_eq!(ledger.len(), 0);
        assert_eq!(ledger.tip_hash(), genesis_hash());
    }

    #[test]
    fn test_ledger_append_single() {
        let mut ledger = Ledger::new();
        let attestation = AttestationBytes::new(vec![1, 2, 3]);
        let entry = Entry::new(genesis_hash(), 0, 1000, attestation);

        ledger.append_entry(entry.clone()).unwrap();

        assert_eq!(ledger.len(), 1);
        assert_ne!(ledger.tip_hash(), genesis_hash());
        assert_eq!(ledger.tip_hash(), entry.hash());
    }

    #[test]
    fn test_ledger_append_multiple() {
        let mut ledger = Ledger::new();

        let e0 = Entry::new(genesis_hash(), 0, 1000, AttestationBytes::new(vec![1]));
        ledger.append_entry(e0.clone()).unwrap();

        let e1_hash = e0.hash();
        let e1 = Entry::new(e1_hash, 1, 2000, AttestationBytes::new(vec![2]));
        ledger.append_entry(e1.clone()).unwrap();

        let e2_hash = e1.hash();
        let e2 = Entry::new(e2_hash, 2, 3000, AttestationBytes::new(vec![3]));
        ledger.append_entry(e2.clone()).unwrap();

        assert_eq!(ledger.len(), 3);
        assert_eq!(ledger.tip_hash(), e2.hash());
    }

    #[test]
    fn test_ledger_append_wrong_sequence() {
        let mut ledger = Ledger::new();
        let e0 = Entry::new(genesis_hash(), 0, 1000, AttestationBytes::new(vec![1]));
        ledger.append_entry(e0).unwrap();

        // Try to append with wrong sequence number
        let e_wrong = Entry::new(genesis_hash(), 5, 2000, AttestationBytes::new(vec![2]));
        let result = ledger.append_entry(e_wrong);
        assert!(result.is_err());
    }

    #[test]
    fn test_ledger_append_wrong_previous_hash() {
        let mut ledger = Ledger::new();
        let e0 = Entry::new(genesis_hash(), 0, 1000, AttestationBytes::new(vec![1]));
        ledger.append_entry(e0.clone()).unwrap();

        // Try to append with wrong previous hash
        let wrong_hash = sha256(b"wrong");
        let e1_wrong = Entry::new(wrong_hash, 1, 2000, AttestationBytes::new(vec![2]));
        let result = ledger.append_entry(e1_wrong);
        assert!(result.is_err());
    }

    #[test]
    fn test_ledger_signed_commit() {
        let mut ledger = Ledger::new();

        // Add first signed entry
        let result = ledger.signed_commit(
            0,
            1000,
            AttestationBytes::new(vec![1, 2, 3]),
            vec![100, 101, 102],
        );
        assert!(result.is_ok());
        assert_eq!(ledger.len(), 1);

        // Add second signed entry
        let result = ledger.signed_commit(
            1,
            2000,
            AttestationBytes::new(vec![4, 5, 6]),
            vec![103, 104, 105],
        );
        assert!(result.is_ok());
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn test_ledger_verify_chain() {
        let mut ledger = Ledger::new();

        let e0 = Entry::new(genesis_hash(), 0, 1000, AttestationBytes::new(vec![1]));
        ledger.append_entry(e0.clone()).unwrap();

        let e1 = Entry::new(e0.hash(), 1, 2000, AttestationBytes::new(vec![2]));
        ledger.append_entry(e1.clone()).unwrap();

        // Verify chain
        let count = ledger.verify_chain().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_ledger_persistence() {
        let mut ledger = Ledger::new();

        let e0 = Entry::new(
            genesis_hash(),
            0,
            1000,
            AttestationBytes::new(vec![1, 2, 3]),
        );
        ledger.append_entry(e0.clone()).unwrap();

        let e1 = Entry::new(e0.hash(), 1, 2000, AttestationBytes::new(vec![4, 5, 6]));
        ledger.append_entry(e1.clone()).unwrap();

        // Persist
        let bytes = ledger.into_bytes();

        // Restore
        let restored = Ledger::from_bytes(bytes).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored.entries().unwrap().len(), 2);
    }

    #[test]
    fn test_ledger_entries_retrieval() {
        let mut ledger = Ledger::new();

        let e0 = Entry::new(genesis_hash(), 0, 1000, AttestationBytes::new(vec![1]));
        ledger.append_entry(e0.clone()).unwrap();

        let e1 = Entry::new(e0.hash(), 1, 2000, AttestationBytes::new(vec![2]));
        ledger.append_entry(e1.clone()).unwrap();

        let entries = ledger.entries().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence(), 0);
        assert_eq!(entries[1].sequence(), 1);
    }

    // Tamper-detection tests
    #[test]
    fn test_ledger_tamper_detection_modified_attestation() {
        let mut ledger = Ledger::new();

        let e0 = Entry::new(genesis_hash(), 0, 1000, AttestationBytes::new(vec![1]));
        let original_hash = e0.hash();
        ledger.append_entry(e0).unwrap();

        // Create a modified entry to verify hash changes
        let e_tampered = Entry::new(
            genesis_hash(),
            0,
            1000,
            AttestationBytes::new(vec![2]), // Changed data
        );
        assert_ne!(e_tampered.hash(), original_hash);
    }

    #[test]
    fn test_ledger_empty_verification() {
        let mut ledger = Ledger::new();
        let count = ledger.verify_chain().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_ledger_large_chain() {
        let mut ledger = Ledger::new();

        let mut prev_hash = genesis_hash();

        // Add 100 entries
        for i in 0..100 {
            let entry = Entry::new(
                prev_hash,
                i,
                1000 + i * 100,
                AttestationBytes::new(vec![i as u8]),
            );
            prev_hash = entry.hash();
            ledger.append_entry(entry).unwrap();
        }

        assert_eq!(ledger.len(), 100);

        // Verify chain
        let count = ledger.verify_chain().unwrap();
        assert_eq!(count, 100);
    }

    #[test]
    fn test_ledger_signed_chain() {
        let mut ledger = Ledger::new();

        let mut prev_hash = genesis_hash();

        for i in 0..5 {
            let entry = Entry::with_signature(
                prev_hash,
                i,
                1000 + i * 100,
                AttestationBytes::new(vec![i as u8]),
                vec![200 + i as u8],
            );
            prev_hash = entry.hash();
            ledger.append_entry(entry).unwrap();
        }

        assert_eq!(ledger.len(), 5);

        // Verify all entries are signed
        let entries = ledger.entries().unwrap();
        for entry in entries {
            assert!(entry.is_signed());
        }
    }
}
