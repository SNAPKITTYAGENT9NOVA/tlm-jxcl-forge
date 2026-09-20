// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

use crate::id::{AccountId, ResourceId};
use crate::region::RegionId;
use cloud_errors::CloudError;
use std::fmt;
use std::str::FromStr;

fn validate_segment(what: &'static str, value: &str) -> Result<(), CloudError> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        return Err(CloudError::InvalidFormat {
            what,
            value: value.to_string(),
            reason: "must be non-empty, lowercase alphanumerics and '-' only".to_string(),
        });
    }
    Ok(())
}

/// This system's internal resource-name format, deliberately shaped
/// like an AWS ARN:
///
/// ```text
/// jxcl:cloud:<partition>:<service>:<region>:<account>:<resource>
/// ```
///
/// `region` and `account` may be empty (a global resource, e.g. an
/// account itself, has no region; a resource not yet assigned to an
/// account has no account) -- everything else is required.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arn {
    pub partition: String,
    pub service: String,
    pub region: Option<RegionId>,
    pub account: Option<AccountId>,
    pub resource: ResourceId,
}

impl Arn {
    pub fn new(
        partition: impl Into<String>,
        service: impl Into<String>,
        region: Option<RegionId>,
        account: Option<AccountId>,
        resource: ResourceId,
    ) -> Result<Self, CloudError> {
        let partition = partition.into();
        let service = service.into();
        validate_segment("arn partition", &partition)?;
        validate_segment("arn service", &service)?;
        Ok(Arn {
            partition,
            service,
            region,
            account,
            resource,
        })
    }
}

impl fmt::Display for Arn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "jxcl:cloud:{}:{}:{}:{}:{}",
            self.partition,
            self.service,
            self.region.as_ref().map(RegionId::as_str).unwrap_or(""),
            self.account.as_ref().map(AccountId::as_str).unwrap_or(""),
            self.resource,
        )
    }
}

impl FromStr for Arn {
    type Err = CloudError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = |reason: &str| CloudError::InvalidFormat {
            what: "arn",
            value: s.to_string(),
            reason: reason.to_string(),
        };

        let parts: Vec<&str> = s.splitn(7, ':').collect();
        let [scheme, cloud, partition, service, region, account, resource] = parts.as_slice()
        else {
            return Err(bad(
                "must have 7 ':'-separated segments: jxcl:cloud:<partition>:<service>:<region>:<account>:<resource>",
            ));
        };

        if *scheme != "jxcl" || *cloud != "cloud" {
            return Err(bad("must start with 'jxcl:cloud:'"));
        }

        let region = if region.is_empty() {
            None
        } else {
            Some(RegionId::new(*region)?)
        };
        let account = if account.is_empty() {
            None
        } else {
            Some(AccountId::new(*account)?)
        };
        let resource = ResourceId::new(*resource)?;

        Arn::new(*partition, *service, region, account, resource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Arn {
        Arn::new(
            "core",
            "compute",
            Some(RegionId::new("us-west-1").unwrap()),
            Some(AccountId::new("000000000001").unwrap()),
            ResourceId::new("instance/i-000001").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn display_matches_the_documented_format() {
        assert_eq!(
            sample().to_string(),
            "jxcl:cloud:core:compute:us-west-1:000000000001:instance/i-000001"
        );
    }

    #[test]
    fn display_then_parse_round_trips() {
        let arn = sample();
        let parsed: Arn = arn.to_string().parse().unwrap();
        assert_eq!(arn, parsed);
    }

    #[test]
    fn region_and_account_may_be_empty_for_global_resources() {
        let arn = Arn::new(
            "core",
            "identity",
            None,
            None,
            ResourceId::new("account/000000000001").unwrap(),
        )
        .unwrap();
        assert_eq!(
            arn.to_string(),
            "jxcl:cloud:core:identity:::account/000000000001"
        );
        let parsed: Arn = arn.to_string().parse().unwrap();
        assert_eq!(arn, parsed);
        assert!(parsed.region.is_none());
        assert!(parsed.account.is_none());
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        assert!("aws:cloud:core:compute:us-west-1:000000000001:i-1"
            .parse::<Arn>()
            .is_err());
    }

    #[test]
    fn parse_rejects_too_few_segments() {
        assert!("jxcl:cloud:core:compute".parse::<Arn>().is_err());
    }

    #[test]
    fn parse_rejects_invalid_region_segment() {
        assert!("jxcl:cloud:core:compute:US-WEST-1:000000000001:i-1"
            .parse::<Arn>()
            .is_err());
    }

    #[test]
    fn resource_segment_may_itself_contain_colons() {
        // splitn(7, ':') means anything past the 6th colon belongs to
        // the resource segment verbatim.
        let s = "jxcl:cloud:core:compute:us-west-1:000000000001:i-1/sub:thing";
        // ':' is rejected by ResourceId validation, so this specific
        // case is expected to fail -- documenting that resource ids
        // use '/' as their sub-delimiter, not ':'.
        assert!(s.parse::<Arn>().is_err());
    }

    #[test]
    fn new_rejects_uppercase_partition_or_service() {
        let resource = ResourceId::new("i-1").unwrap();
        assert!(Arn::new("Core", "compute", None, None, resource.clone()).is_err());
        assert!(Arn::new("core", "Compute", None, None, resource).is_err());
    }
}
