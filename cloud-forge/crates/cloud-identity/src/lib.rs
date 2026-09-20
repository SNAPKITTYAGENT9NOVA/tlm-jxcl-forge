//! A minimal [`Principal`] type: enough for `cloud-policy` to evaluate
//! access against. The full IAM surface -- credentials, sessions,
//! federation, policy documents -- is Phase 13 in the roadmap and is
//! deliberately not pulled forward into this phase.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::ResourceId;
use std::fmt;
use std::str::FromStr;

/// Who is attempting an action: a human user, an assumed role, or a
/// service acting on its own behalf.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Principal {
    User(ResourceId),
    Role(ResourceId),
    Service(ResourceId),
}

impl Principal {
    /// The kind name used in this type's canonical string form.
    pub fn kind(&self) -> &'static str {
        match self {
            Principal::User(_) => "user",
            Principal::Role(_) => "role",
            Principal::Service(_) => "service",
        }
    }

    pub fn id(&self) -> &ResourceId {
        match self {
            Principal::User(id) | Principal::Role(id) | Principal::Service(id) => id,
        }
    }
}

impl fmt::Display for Principal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.kind(), self.id())
    }
}

impl FromStr for Principal {
    type Err = CloudError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || CloudError::InvalidFormat {
            what: "principal",
            value: s.to_string(),
            reason: "must look like 'user/<id>', 'role/<id>', or 'service/<id>'".to_string(),
        };
        let (kind, id) = s.split_once('/').ok_or_else(bad)?;
        let id = ResourceId::new(id)?;
        match kind {
            "user" => Ok(Principal::User(id)),
            "role" => Ok(Principal::Role(id)),
            "service" => Ok(Principal::Service(id)),
            _ => Err(bad()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    #[test]
    fn display_format_is_kind_slash_id() {
        assert_eq!(Principal::User(id("alice")).to_string(), "user/alice");
        assert_eq!(Principal::Role(id("admin")).to_string(), "role/admin");
        assert_eq!(
            Principal::Service(id("scheduler")).to_string(),
            "service/scheduler"
        );
    }

    #[test]
    fn display_then_parse_round_trips_for_every_kind() {
        for p in [
            Principal::User(id("alice")),
            Principal::Role(id("admin")),
            Principal::Service(id("scheduler")),
        ] {
            let parsed: Principal = p.to_string().parse().unwrap();
            assert_eq!(p, parsed);
        }
    }

    #[test]
    fn parse_rejects_unknown_kind() {
        assert!("robot/alice".parse::<Principal>().is_err());
    }

    #[test]
    fn parse_rejects_missing_slash() {
        assert!("alice".parse::<Principal>().is_err());
    }

    #[test]
    fn kind_and_id_accessors_match_the_variant() {
        let p = Principal::Role(id("admin"));
        assert_eq!(p.kind(), "role");
        assert_eq!(p.id(), &id("admin"));
    }
}
