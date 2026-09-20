use cloud_errors::CloudError;
use std::fmt;
use std::str::FromStr;

/// Splits `us-west-1`-shaped strings into (`"us-west"`, `1`), the
/// validation both [`RegionId`] and [`AzId`] share.
fn split_region_shape(value: &str) -> Option<(String, u32)> {
    let (prefix, number) = value.rsplit_once('-')?;
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        return None;
    }
    if prefix.starts_with('-') || prefix.ends_with('-') || prefix.contains("--") {
        return None;
    }
    let number: u32 = number.parse().ok()?;
    Some((prefix.to_string(), number))
}

/// A region identifier shaped like `us-west-1`: one or more
/// lowercase, hyphen-separated words followed by a hyphen and a
/// non-negative integer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionId(String);

impl RegionId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudError> {
        let value = value.into();
        if split_region_shape(&value).is_none() {
            return Err(CloudError::InvalidFormat {
                what: "region id",
                value,
                reason:
                    "must look like 'us-west-1': lowercase words joined by '-', ending in a number"
                        .to_string(),
            });
        }
        Ok(RegionId(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for RegionId {
    type Err = CloudError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RegionId::new(s)
    }
}

/// An availability zone identifier shaped like `us-west-1a`: a valid
/// [`RegionId`] followed by a single lowercase letter suffix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AzId(String);

impl AzId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudError> {
        let value = value.into();
        let mut chars = value.chars();
        let Some(suffix) = chars.next_back() else {
            return Err(CloudError::InvalidFormat {
                what: "availability zone id",
                value,
                reason: "must not be empty".to_string(),
            });
        };
        if !suffix.is_ascii_lowercase() {
            return Err(CloudError::InvalidFormat {
                what: "availability zone id",
                value,
                reason: "must end in a single lowercase letter suffix".to_string(),
            });
        }
        let region_part = chars.as_str();
        if RegionId::new(region_part).is_err() {
            return Err(CloudError::InvalidFormat {
                what: "availability zone id",
                value,
                reason: "the part before the trailing letter must be a valid region id".to_string(),
            });
        }
        Ok(AzId(value))
    }

    /// The region this availability zone belongs to.
    pub fn region(&self) -> RegionId {
        let mut chars = self.0.chars();
        chars.next_back();
        RegionId::new(chars.as_str()).expect("validated at construction")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AzId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AzId {
    type Err = CloudError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AzId::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_id_accepts_typical_shapes() {
        assert!(RegionId::new("us-west-1").is_ok());
        assert!(RegionId::new("eu-central-1").is_ok());
        assert!(RegionId::new("us-gov-west-1").is_ok());
    }

    #[test]
    fn region_id_rejects_missing_number_or_uppercase() {
        assert!(RegionId::new("us-west").is_err());
        assert!(RegionId::new("US-WEST-1").is_err());
        assert!(RegionId::new("").is_err());
    }

    #[test]
    fn region_id_display_and_fromstr_round_trip() {
        let r = RegionId::new("us-west-1").unwrap();
        let parsed: RegionId = r.to_string().parse().unwrap();
        assert_eq!(r, parsed);
    }

    #[test]
    fn az_id_accepts_region_plus_letter() {
        assert!(AzId::new("us-west-1a").is_ok());
        assert!(AzId::new("us-west-1b").is_ok());
    }

    #[test]
    fn az_id_rejects_missing_letter_or_invalid_region_part() {
        assert!(AzId::new("us-west-1").is_err()); // no letter suffix beyond the digit
        assert!(AzId::new("US-WEST-1a").is_err());
        assert!(AzId::new("a").is_err());
    }

    #[test]
    fn az_id_region_extracts_the_owning_region() {
        let az = AzId::new("us-west-1a").unwrap();
        assert_eq!(az.region(), RegionId::new("us-west-1").unwrap());
    }
}
