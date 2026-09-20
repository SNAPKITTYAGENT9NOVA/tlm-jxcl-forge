use cloud_errors::CloudError;
use std::fmt;
use std::str::FromStr;

const MAX_ID_LEN: usize = 128;

fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.'
}

/// An opaque, validated resource identifier (e.g. `instance/i-000001`).
///
/// Validation is deliberately permissive about *shape* (any non-empty
/// run of alphanumerics, `-`, `_`, `/`, `.`) since resource ids are
/// minted by many future services with different conventions; it is
/// strict about *what's forbidden*: no whitespace, no control
/// characters, nothing that would break the `Arn` format's `:`
/// delimiter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CloudError::InvalidFormat {
                what: "resource id",
                value,
                reason: "must not be empty".to_string(),
            });
        }
        if value.len() > MAX_ID_LEN {
            return Err(CloudError::InvalidFormat {
                what: "resource id",
                value,
                reason: format!("must be at most {MAX_ID_LEN} characters"),
            });
        }
        if value.contains(':') || !value.chars().all(is_id_char) {
            return Err(CloudError::InvalidFormat {
                what: "resource id",
                value,
                reason: "must contain only ASCII alphanumerics, '-', '_', '/', '.'".to_string(),
            });
        }
        Ok(ResourceId(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ResourceId {
    type Err = CloudError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ResourceId::new(s)
    }
}

/// A resource's kind, as a validated, extensible, kebab-case string
/// (e.g. `compute-instance`, `object-store-bucket`). Deliberately not
/// a closed Rust enum: new services add new resource types, and this
/// crate should never need editing when they do.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceType(String);

impl ResourceType {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !value.starts_with('-')
            && !value.ends_with('-');
        if !valid {
            return Err(CloudError::InvalidFormat {
                what: "resource type",
                value,
                reason:
                    "must be non-empty, lowercase kebab-case (a-z0-9-), no leading/trailing '-'"
                        .to_string(),
            });
        }
        Ok(ResourceType(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ResourceType {
    type Err = CloudError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ResourceType::new(s)
    }
}

/// A 12-digit account identifier (mirroring AWS's own convention: a
/// fixed-width numeric id that is never reused and never arithmetic).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(String);

impl AccountId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudError> {
        let value = value.into();
        if value.len() != 12 || !value.chars().all(|c| c.is_ascii_digit()) {
            return Err(CloudError::InvalidFormat {
                what: "account id",
                value,
                reason: "must be exactly 12 ASCII digits".to_string(),
            });
        }
        Ok(AccountId(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AccountId {
    type Err = CloudError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AccountId::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_accepts_typical_shapes() {
        assert!(ResourceId::new("instance/i-000001").is_ok());
        assert!(ResourceId::new("bucket.name-1").is_ok());
    }

    #[test]
    fn resource_id_rejects_empty() {
        assert!(ResourceId::new("").is_err());
    }

    #[test]
    fn resource_id_rejects_colon_and_whitespace() {
        assert!(ResourceId::new("has:colon").is_err());
        assert!(ResourceId::new("has space").is_err());
    }

    #[test]
    fn resource_id_rejects_too_long() {
        let long = "a".repeat(MAX_ID_LEN + 1);
        assert!(ResourceId::new(long).is_err());
    }

    #[test]
    fn resource_id_display_and_fromstr_round_trip() {
        let id = ResourceId::new("i-000001").unwrap();
        let s = id.to_string();
        let parsed: ResourceId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn resource_type_accepts_kebab_case() {
        assert!(ResourceType::new("compute-instance").is_ok());
        assert!(ResourceType::new("bucket").is_ok());
    }

    #[test]
    fn resource_type_rejects_uppercase_and_edge_hyphens() {
        assert!(ResourceType::new("Compute-Instance").is_err());
        assert!(ResourceType::new("-leading").is_err());
        assert!(ResourceType::new("trailing-").is_err());
        assert!(ResourceType::new("").is_err());
    }

    #[test]
    fn account_id_requires_exactly_twelve_digits() {
        assert!(AccountId::new("000000000001").is_ok());
        assert!(AccountId::new("00000000001").is_err()); // 11 digits
        assert!(AccountId::new("0000000000012").is_err()); // 13 digits
        assert!(AccountId::new("00000000000a").is_err()); // non-digit
    }

    #[test]
    fn account_id_display_and_fromstr_round_trip() {
        let id = AccountId::new("123456789012").unwrap();
        let parsed: AccountId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }
}
