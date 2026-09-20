// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A validated key/value tag map, shared by every cloud-forge
//! resource. Backed by a `BTreeMap` rather than a hash map so
//! iteration order is deterministic (sorted by key) -- this workspace
//! never lets iteration order depend on hash-seed randomization.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use std::collections::btree_map;
use std::collections::BTreeMap;

/// The reserved prefix for system-managed tags: user code may never
/// set a tag whose key starts with this, so a future control plane
/// can safely stamp its own tags (e.g. `jxcl:created-by`) without a
/// caller being able to spoof or overwrite them.
pub const RESERVED_KEY_PREFIX: &str = "jxcl:";

pub const MAX_TAG_COUNT: usize = 50;
pub const MAX_KEY_LEN: usize = 128;
pub const MAX_VALUE_LEN: usize = 256;

/// A validated set of resource tags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags(BTreeMap<String, String>);

impl Tags {
    pub fn new() -> Self {
        Tags(BTreeMap::new())
    }

    /// Validates `key`/`value` and inserts them, returning the
    /// previous value if `key` was already present. Overwriting an
    /// existing key never counts against [`MAX_TAG_COUNT`].
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Option<String>, CloudError> {
        let key = key.into();
        let value = value.into();
        self.validate_key(&key)?;
        if value.chars().count() > MAX_VALUE_LEN {
            return Err(CloudError::InvalidFormat {
                what: "tag value",
                value,
                reason: format!("must be at most {MAX_VALUE_LEN} characters"),
            });
        }
        if !self.0.contains_key(&key) && self.0.len() >= MAX_TAG_COUNT {
            return Err(CloudError::QuotaExceeded {
                resource: "tags".to_string(),
                limit: MAX_TAG_COUNT as u64,
                requested: (self.0.len() + 1) as u64,
            });
        }
        Ok(self.0.insert(key, value))
    }

    fn validate_key(&self, key: &str) -> Result<(), CloudError> {
        if key.is_empty() {
            return Err(CloudError::InvalidFormat {
                what: "tag key",
                value: key.to_string(),
                reason: "must not be empty".to_string(),
            });
        }
        if key.chars().count() > MAX_KEY_LEN {
            return Err(CloudError::InvalidFormat {
                what: "tag key",
                value: key.to_string(),
                reason: format!("must be at most {MAX_KEY_LEN} characters"),
            });
        }
        if key.starts_with(RESERVED_KEY_PREFIX) {
            return Err(CloudError::InvalidFormat {
                what: "tag key",
                value: key.to_string(),
                reason: format!("must not start with the reserved prefix {RESERVED_KEY_PREFIX:?}"),
            });
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates tags in deterministic (key-sorted) order.
    pub fn iter(&self) -> btree_map::Iter<'_, String, String> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = (&'a String, &'a String);
    type IntoIter = btree_map::Iter<'a, String, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_round_trip() {
        let mut tags = Tags::new();
        tags.insert("env", "prod").unwrap();
        assert_eq!(tags.get("env"), Some("prod"));
    }

    #[test]
    fn overwriting_an_existing_key_returns_the_previous_value() {
        let mut tags = Tags::new();
        tags.insert("env", "staging").unwrap();
        let previous = tags.insert("env", "prod").unwrap();
        assert_eq!(previous, Some("staging".to_string()));
        assert_eq!(tags.get("env"), Some("prod"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn rejects_empty_key() {
        let mut tags = Tags::new();
        assert!(tags.insert("", "x").is_err());
    }

    #[test]
    fn rejects_key_or_value_over_the_length_limit() {
        let mut tags = Tags::new();
        let long_key = "k".repeat(MAX_KEY_LEN + 1);
        assert!(tags.insert(long_key, "v").is_err());
        let long_value = "v".repeat(MAX_VALUE_LEN + 1);
        assert!(tags.insert("k", long_value).is_err());
    }

    #[test]
    fn rejects_reserved_key_prefix() {
        let mut tags = Tags::new();
        assert!(tags.insert("jxcl:created-by", "control-plane").is_err());
    }

    #[test]
    fn rejects_exceeding_max_tag_count() {
        let mut tags = Tags::new();
        for i in 0..MAX_TAG_COUNT {
            tags.insert(format!("k{i}"), "v").unwrap();
        }
        assert_eq!(tags.len(), MAX_TAG_COUNT);
        assert!(tags.insert("one-too-many", "v").is_err());
    }

    #[test]
    fn overwriting_at_the_limit_still_succeeds() {
        let mut tags = Tags::new();
        for i in 0..MAX_TAG_COUNT {
            tags.insert(format!("k{i}"), "v").unwrap();
        }
        // Overwriting an existing key at the limit must not be
        // rejected as if it were growing the set.
        assert!(tags.insert("k0", "new-value").is_ok());
    }

    #[test]
    fn iteration_order_is_sorted_by_key() {
        let mut tags = Tags::new();
        tags.insert("zeta", "1").unwrap();
        tags.insert("alpha", "2").unwrap();
        tags.insert("mid", "3").unwrap();
        let keys: Vec<&str> = tags.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn remove_returns_the_removed_value() {
        let mut tags = Tags::new();
        tags.insert("env", "prod").unwrap();
        assert_eq!(tags.remove("env"), Some("prod".to_string()));
        assert!(tags.is_empty());
    }
}
