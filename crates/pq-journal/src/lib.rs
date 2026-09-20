// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! An append-only, length-prefixed write-ahead log file format with replay, for durability.
//!
//! # Overview
//!
//! `pq-journal` provides a write-ahead journal (WAL) for transactional storage operations.
//! It ensures durability by recording operations in an append-only log before they are applied.
//!
//! # Key Features
//!
//! - **Append-only semantics**: Entries can only be added, not modified or deleted in place
//! - **Length-prefixed format**: Each entry is prefixed with its length for reliable parsing
//! - **Crash recovery**: Supports scanning the log and recovering partial/complete entries
//! - **Truncation**: Clean removal of entries up to a checkpoint
//!
//! # Format
//!
//! Each entry in the journal is stored as:
//! ```text
//! [4-byte length: u32][entry_data]
//! ```
//!
//! The length field encodes the size of the entry_data in bytes, allowing recovery from
//! crashes at entry boundaries.
//!
//! Owns: Journal::append/replay.
#![forbid(unsafe_code)]

use jxcl_bytes::{ByteCursor, ByteCursorMut};
use std::fmt;
use std::io;

/// Error type for journal operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    /// IO operation failed (file read/write).
    Io(String),
    /// Entry is corrupted or incomplete.
    Corrupted(String),
    /// Invalid entry format.
    InvalidEntry(String),
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::Io(msg) => write!(f, "io error: {}", msg),
            JournalError::Corrupted(msg) => write!(f, "corrupted entry: {}", msg),
            JournalError::InvalidEntry(msg) => write!(f, "invalid entry: {}", msg),
        }
    }
}

impl std::error::Error for JournalError {}

impl From<io::Error> for JournalError {
    fn from(e: io::Error) -> Self {
        JournalError::Io(e.to_string())
    }
}

/// A single entry in the journal, containing opaque data.
///
/// Entries are stored in a length-prefixed format in the journal's backing storage.
/// The actual serialization/deserialization of entry data is the responsibility
/// of the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// The opaque entry data.
    data: Vec<u8>,
}

impl JournalEntry {
    /// Create a new journal entry from raw bytes.
    pub fn new(data: Vec<u8>) -> Self {
        JournalEntry { data }
    }

    /// Borrow the entry's data.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Consume this entry and return its data.
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// Return the length of the entry's data.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Return whether the entry is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Serialize this entry to a byte buffer in length-prefixed format.
    ///
    /// Format: [4-byte length (u32)][data]
    fn to_bytes(&self) -> Result<Vec<u8>, JournalError> {
        let mut cursor = ByteCursorMut::new();
        cursor.write_u32(self.data.len() as u32).map_err(|e| {
            JournalError::Io(format!("failed to write entry length: {}", e))
        })?;
        cursor.write_bytes(&self.data).map_err(|e| {
            JournalError::Io(format!("failed to write entry data: {}", e))
        })?;
        Ok(cursor.into_vec())
    }

    /// Deserialize a single entry from a buffer, consuming the length prefix and data.
    ///
    /// Returns the parsed entry and the number of bytes consumed.
    fn from_bytes(data: &[u8]) -> Result<(JournalEntry, usize), JournalError> {
        if data.len() < 4 {
            return Err(JournalError::Corrupted(
                "entry header too short (need 4 bytes for length)".to_string(),
            ));
        }

        let mut cursor = ByteCursor::new(data);
        let entry_len = cursor.read_u32().map_err(|e| {
            JournalError::Io(format!("failed to read entry length: {}", e))
        })? as usize;

        if cursor.remaining() < entry_len {
            return Err(JournalError::Corrupted(format!(
                "incomplete entry data: expected {} bytes, got {}",
                entry_len,
                cursor.remaining()
            )));
        }

        let entry_data = cursor.read_bytes(entry_len).map_err(|e| {
            JournalError::Io(format!("failed to read entry data: {}", e))
        })?;

        let entry = JournalEntry {
            data: entry_data.to_vec(),
        };

        // Return the entry and how many bytes we consumed (4 for length + entry_len)
        Ok((entry, 4 + entry_len))
    }
}

/// An append-only journal for write-ahead logging.
///
/// The journal stores entries in a length-prefixed format in a buffer (or file).
/// It supports appending new entries, scanning existing entries, and truncating
/// the log up to a checkpoint.
#[derive(Debug, Clone)]
pub struct Journal {
    /// In-memory buffer storing all journal entries.
    buffer: Vec<u8>,
}

impl Journal {
    /// Create a new, empty journal.
    pub fn new() -> Self {
        Journal {
            buffer: Vec::new(),
        }
    }

    /// Create a journal from existing bytes (e.g., loaded from disk).
    ///
    /// This validates that the buffer contains well-formed entries.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, JournalError> {
        // Validate the buffer by scanning all entries
        let mut cursor = 0;
        while cursor < data.len() {
            if data.len() - cursor < 4 {
                return Err(JournalError::Corrupted(
                    "incomplete entry header at end of journal".to_string(),
                ));
            }

            let mut temp_cursor = ByteCursor::new(&data[cursor..]);
            let entry_len = temp_cursor.read_u32().map_err(|e| {
                JournalError::Io(format!("failed to read entry length: {}", e))
            })? as usize;

            if temp_cursor.remaining() < entry_len {
                return Err(JournalError::Corrupted(format!(
                    "incomplete entry data: expected {} bytes, got {}",
                    entry_len,
                    temp_cursor.remaining()
                )));
            }

            cursor += 4 + entry_len;
        }

        Ok(Journal { buffer: data })
    }

    /// Append a new entry to the journal.
    pub fn append(&mut self, entry: JournalEntry) -> Result<(), JournalError> {
        let serialized = entry.to_bytes()?;
        self.buffer.extend_from_slice(&serialized);
        Ok(())
    }

    /// Append multiple entries to the journal.
    pub fn append_batch(&mut self, entries: Vec<JournalEntry>) -> Result<(), JournalError> {
        for entry in entries {
            self.append(entry)?;
        }
        Ok(())
    }

    /// Scan all entries in the journal.
    ///
    /// Returns a vector of all entries in order, or an error if the journal is corrupted.
    pub fn scan(&self) -> Result<Vec<JournalEntry>, JournalError> {
        let mut entries = Vec::new();
        let mut cursor = 0;

        while cursor < self.buffer.len() {
            let (entry, bytes_consumed) = JournalEntry::from_bytes(&self.buffer[cursor..])?;
            entries.push(entry);
            cursor += bytes_consumed;
        }

        Ok(entries)
    }

    /// Scan entries and apply a callback to each one.
    ///
    /// This is a streaming version of scan that doesn't allocate all entries
    /// in memory at once. The callback is called for each entry in order.
    /// If the callback returns an error, scanning stops and the error is returned.
    pub fn scan_with<F>(&self, mut callback: F) -> Result<(), JournalError>
    where
        F: FnMut(&JournalEntry) -> Result<(), JournalError>,
    {
        let mut cursor = 0;

        while cursor < self.buffer.len() {
            let (entry, bytes_consumed) = JournalEntry::from_bytes(&self.buffer[cursor..])?;
            callback(&entry)?;
            cursor += bytes_consumed;
        }

        Ok(())
    }

    /// Truncate the journal, removing all entries up to (and including) the
    /// entry at the given position.
    ///
    /// `position` is the byte offset after which to keep data. This is typically
    /// obtained from a checkpoint mechanism that tracks progress through entries.
    pub fn truncate(&mut self, position: usize) {
        if position >= self.buffer.len() {
            self.buffer.clear();
        } else {
            self.buffer.drain(..position);
        }
    }

    /// Get the current size of the journal buffer in bytes.
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Get the raw buffer contents. Useful for persistence.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume this journal and return its buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Clear all entries from the journal.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Check if the journal is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for Journal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_entry_create() {
        let data = vec![1, 2, 3, 4, 5];
        let entry = JournalEntry::new(data.clone());
        assert_eq!(entry.as_slice(), &data);
        assert_eq!(entry.len(), 5);
        assert!(!entry.is_empty());
    }

    #[test]
    fn test_journal_entry_empty() {
        let entry = JournalEntry::new(Vec::new());
        assert!(entry.is_empty());
        assert_eq!(entry.len(), 0);
    }

    #[test]
    fn test_journal_entry_serialization() {
        let data = vec![10, 20, 30];
        let entry = JournalEntry::new(data.clone());
        let serialized = entry.to_bytes().unwrap();

        // Should be 4 bytes (length) + 3 bytes (data)
        assert_eq!(serialized.len(), 7);

        // First 4 bytes should encode the length 3
        let mut cursor = ByteCursor::new(&serialized);
        let len = cursor.read_u32().unwrap();
        assert_eq!(len, 3);

        let entry_data = cursor.read_bytes(3).unwrap();
        assert_eq!(entry_data, &data);
    }

    #[test]
    fn test_journal_entry_deserialization() {
        let original_data = vec![10, 20, 30];
        let entry = JournalEntry::new(original_data.clone());
        let serialized = entry.to_bytes().unwrap();

        let (deserialized, consumed) = JournalEntry::from_bytes(&serialized).unwrap();
        assert_eq!(deserialized.as_slice(), &original_data);
        assert_eq!(consumed, 7); // 4 + 3
    }

    #[test]
    fn test_journal_create_empty() {
        let journal = Journal::new();
        assert!(journal.is_empty());
        assert_eq!(journal.size(), 0);
    }

    #[test]
    fn test_journal_append_single() {
        let mut journal = Journal::new();
        let entry = JournalEntry::new(vec![1, 2, 3]);

        journal.append(entry.clone()).unwrap();

        assert!(!journal.is_empty());
        let entries = journal.scan().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], entry);
    }

    #[test]
    fn test_journal_append_multiple() {
        let mut journal = Journal::new();
        let entry1 = JournalEntry::new(vec![1, 2, 3]);
        let entry2 = JournalEntry::new(vec![4, 5, 6, 7]);
        let entry3 = JournalEntry::new(vec![8]);

        journal.append(entry1.clone()).unwrap();
        journal.append(entry2.clone()).unwrap();
        journal.append(entry3.clone()).unwrap();

        let entries = journal.scan().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], entry1);
        assert_eq!(entries[1], entry2);
        assert_eq!(entries[2], entry3);
    }

    #[test]
    fn test_journal_append_batch() {
        let mut journal = Journal::new();
        let entries = vec![
            JournalEntry::new(vec![1, 2, 3]),
            JournalEntry::new(vec![4, 5, 6, 7]),
            JournalEntry::new(vec![8]),
        ];

        journal.append_batch(entries.clone()).unwrap();

        let scanned = journal.scan().unwrap();
        assert_eq!(scanned.len(), 3);
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(scanned[i], *entry);
        }
    }

    #[test]
    fn test_journal_truncate_full() {
        let mut journal = Journal::new();
        journal.append(JournalEntry::new(vec![1, 2, 3])).unwrap();
        journal.append(JournalEntry::new(vec![4, 5])).unwrap();

        let size = journal.size();
        journal.truncate(size);

        assert!(journal.is_empty());
    }

    #[test]
    fn test_journal_truncate_partial() {
        let mut journal = Journal::new();
        let entry1 = JournalEntry::new(vec![1, 2, 3]);
        let entry2 = JournalEntry::new(vec![4, 5, 6, 7]);

        journal.append(entry1).unwrap();
        journal.append(entry2.clone()).unwrap();

        // Truncate after the first entry (4 bytes for length + 3 bytes for data)
        journal.truncate(7);

        let entries = journal.scan().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], entry2);
    }

    #[test]
    fn test_journal_clear() {
        let mut journal = Journal::new();
        journal.append(JournalEntry::new(vec![1, 2, 3])).unwrap();
        journal.append(JournalEntry::new(vec![4, 5])).unwrap();

        assert!(!journal.is_empty());
        journal.clear();
        assert!(journal.is_empty());
    }

    #[test]
    fn test_journal_from_bytes() {
        let mut journal = Journal::new();
        let entry1 = JournalEntry::new(vec![1, 2, 3]);
        let entry2 = JournalEntry::new(vec![4, 5, 6]);

        journal.append(entry1.clone()).unwrap();
        journal.append(entry2.clone()).unwrap();

        let bytes = journal.into_bytes();
        let restored = Journal::from_bytes(bytes).unwrap();

        let entries = restored.scan().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], entry1);
        assert_eq!(entries[1], entry2);
    }

    #[test]
    fn test_journal_scan_with() {
        let mut journal = Journal::new();
        let entry1 = JournalEntry::new(vec![1, 2, 3]);
        let entry2 = JournalEntry::new(vec![4, 5, 6]);

        journal.append(entry1).unwrap();
        journal.append(entry2).unwrap();

        let mut count = 0;
        journal
            .scan_with(|_entry| {
                count += 1;
                Ok(())
            })
            .unwrap();

        assert_eq!(count, 2);
    }

    // Boundary tests
    #[test]
    fn test_journal_entry_large_data() {
        let large_data = vec![42u8; 1_000_000]; // 1MB
        let entry = JournalEntry::new(large_data.clone());
        assert_eq!(entry.len(), 1_000_000);

        let serialized = entry.to_bytes().unwrap();
        let (deserialized, _) = JournalEntry::from_bytes(&serialized).unwrap();
        assert_eq!(deserialized.as_slice(), &large_data);
    }

    #[test]
    fn test_journal_many_entries() {
        let mut journal = Journal::new();

        // Add 1000 entries
        for i in 0..1000 {
            let data = vec![(i % 256) as u8; i % 100 + 1];
            journal.append(JournalEntry::new(data)).unwrap();
        }

        let entries = journal.scan().unwrap();
        assert_eq!(entries.len(), 1000);
    }

    #[test]
    fn test_journal_entry_corrupted_header() {
        let data = vec![1, 2]; // Too short to contain a 4-byte length
        let result = JournalEntry::from_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_journal_entry_corrupted_data() {
        let mut data = vec![0, 0, 0, 10]; // Says 10 bytes follow
        data.extend_from_slice(&[1, 2, 3]); // But only 3 bytes provided

        let result = JournalEntry::from_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_journal_from_corrupted_bytes() {
        let mut data = vec![0, 0, 0, 5]; // Says 5 bytes follow
        data.extend_from_slice(&[1, 2]); // But only 2 bytes provided

        let result = Journal::from_bytes(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_journal_truncate_boundary_at_entry() {
        let mut journal = Journal::new();
        let e1 = JournalEntry::new(vec![1, 2, 3, 4, 5]);
        let e2 = JournalEntry::new(vec![6, 7, 8]);
        let e3 = JournalEntry::new(vec![9, 10]);

        journal.append(e1).unwrap();
        journal.append(e2).unwrap();
        journal.append(e3.clone()).unwrap();

        // Truncate at position that leaves e3 (9 bytes)
        journal.truncate(9 + 4 + 3); // e1 (4+5) + e2 (4+3)

        let entries = journal.scan().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], e3);
    }

    #[test]
    fn test_journal_persistence_round_trip() {
        let mut journal = Journal::new();
        let entries = vec![
            JournalEntry::new(b"hello world".to_vec()),
            JournalEntry::new(b"foo bar baz".to_vec()),
            JournalEntry::new(vec![0, 1, 2, 255, 254]),
        ];

        for entry in &entries {
            journal.append(entry.clone()).unwrap();
        }

        // Serialize and deserialize
        let bytes = journal.into_bytes();
        let restored = Journal::from_bytes(bytes).unwrap();

        let scanned = restored.scan().unwrap();
        assert_eq!(scanned, entries);
    }
}
