//! A simple address-to-source-line debug info format emitted by the
//! assembler and consumed by the disassembler/debugger.
//!
//! Owns: [`LineTable`] -- the only place a code address is mapped back to
//! the `(file, line)` it was assembled from.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// address -> (source file, 1-based line number).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineTable {
    entries: BTreeMap<u64, (String, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugInfoErrorKind {
    Truncated,
    InvalidUtf8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugInfoError {
    pub kind: DebugInfoErrorKind,
    pub reason: String,
}

impl fmt::Display for DebugInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "debug info error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for DebugInfoError {}

fn truncated(what: &str) -> DebugInfoError {
    DebugInfoError {
        kind: DebugInfoErrorKind::Truncated,
        reason: format!("buffer too short while reading {}", what),
    }
}

impl LineTable {
    pub fn new() -> Self {
        LineTable {
            entries: BTreeMap::new(),
        }
    }

    /// Record that `address` was produced from `line` of `file`. A later
    /// call for the same `address` overwrites the earlier mapping (an
    /// address has exactly one source location).
    pub fn insert(&mut self, address: u64, file: impl Into<String>, line: u32) {
        self.entries.insert(address, (file.into(), line));
    }

    /// Look up the `(file, line)` an address was assembled from.
    pub fn lookup(&self, address: u64) -> Option<(&str, u32)> {
        self.entries.get(&address).map(|(f, l)| (f.as_str(), *l))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u64, &str, u32)> {
        self.entries
            .iter()
            .map(|(addr, (file, line))| (*addr, file.as_str(), *line))
    }

    /// Serialize to a simple, self-contained binary format:
    /// `entry_count: u32 LE`, then per entry, in ascending address order:
    /// `address: u64 LE`, `line: u32 LE`, `file_len: u16 LE`, `file` bytes
    /// (UTF-8, not NUL-terminated).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + self.entries.len() * 16);
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for (addr, (file, line)) in &self.entries {
            out.extend_from_slice(&addr.to_le_bytes());
            out.extend_from_slice(&line.to_le_bytes());
            let file_bytes = file.as_bytes();
            out.extend_from_slice(&(file_bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(file_bytes);
        }
        out
    }

    /// Inverse of [`LineTable::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DebugInfoError> {
        let mut pos = 0usize;
        let take = |pos: &mut usize, n: usize, what: &str| -> Result<&[u8], DebugInfoError> {
            let end = pos.checked_add(n).ok_or_else(|| truncated(what))?;
            if end > bytes.len() {
                return Err(truncated(what));
            }
            let slice = &bytes[*pos..end];
            *pos = end;
            Ok(slice)
        };

        let count_bytes = take(&mut pos, 4, "entry count")?;
        let count = u32::from_le_bytes(count_bytes.try_into().unwrap());

        let mut table = LineTable::new();
        for _ in 0..count {
            let addr_bytes = take(&mut pos, 8, "address")?;
            let address = u64::from_le_bytes(addr_bytes.try_into().unwrap());
            let line_bytes = take(&mut pos, 4, "line")?;
            let line = u32::from_le_bytes(line_bytes.try_into().unwrap());
            let file_len_bytes = take(&mut pos, 2, "file length")?;
            let file_len = u16::from_le_bytes(file_len_bytes.try_into().unwrap()) as usize;
            let file_bytes = take(&mut pos, file_len, "file name")?;
            let file = String::from_utf8(file_bytes.to_vec()).map_err(|e| DebugInfoError {
                kind: DebugInfoErrorKind::InvalidUtf8,
                reason: e.to_string(),
            })?;
            table.insert(address, file, line);
        }
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_lookup() {
        let mut t = LineTable::new();
        t.insert(0, "loop.jxcl", 2);
        t.insert(5, "loop.jxcl", 3);
        assert_eq!(t.lookup(0), Some(("loop.jxcl", 2)));
        assert_eq!(t.lookup(5), Some(("loop.jxcl", 3)));
        assert_eq!(t.lookup(99), None);
    }

    #[test]
    fn later_insert_overwrites_same_address() {
        let mut t = LineTable::new();
        t.insert(0, "a.jxcl", 1);
        t.insert(0, "a.jxcl", 2);
        assert_eq!(t.lookup(0), Some(("a.jxcl", 2)));
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn roundtrips_through_bytes() {
        let mut t = LineTable::new();
        t.insert(0, "loop.jxcl", 2);
        t.insert(5, "loop.jxcl", 3);
        t.insert(6, "other/name.jxcl", 42);
        let bytes = t.to_bytes();
        let restored = LineTable::from_bytes(&bytes).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn empty_table_roundtrips() {
        let t = LineTable::new();
        let bytes = t.to_bytes();
        assert_eq!(bytes, 0u32.to_le_bytes());
        let restored = LineTable::from_bytes(&bytes).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn rejects_truncated_bytes() {
        let mut t = LineTable::new();
        t.insert(0, "a.jxcl", 1);
        let mut bytes = t.to_bytes();
        bytes.truncate(bytes.len() - 1);
        let err = LineTable::from_bytes(&bytes).unwrap_err();
        assert_eq!(err.kind, DebugInfoErrorKind::Truncated);
    }

    #[test]
    fn iter_yields_ascending_addresses() {
        let mut t = LineTable::new();
        t.insert(10, "a.jxcl", 1);
        t.insert(0, "a.jxcl", 0);
        let addrs: Vec<u64> = t.iter().map(|(a, _, _)| a).collect();
        assert_eq!(addrs, vec![0, 10]);
    }
}
