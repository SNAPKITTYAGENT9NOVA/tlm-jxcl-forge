//! The shared error type returned by every `cloud-forge` crate.
//!
//! Rather than let each crate define its own error enum (and force
//! every caller composing several primitives to write a conversion
//! layer between them), every crate in this workspace returns
//! [`CloudError`] directly. Its variants are deliberately structural
//! (an operation kind plus the specific values involved) rather than
//! one variant per crate, so adding a new primitive crate never
//! requires adding a new variant here.
#![forbid(unsafe_code)]

use std::fmt;

/// A structured error shared across every `cloud-forge` crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudError {
    /// A value failed validation when constructing a typed wrapper
    /// (e.g. an `AccountId` that isn't 12 ASCII digits).
    InvalidFormat {
        what: &'static str,
        value: String,
        reason: String,
    },
    /// A state machine was asked to move from `from` to `to`, and that
    /// transition is not in its allowed set.
    InvalidTransition {
        what: &'static str,
        from: String,
        to: String,
    },
    /// A quota's limit would be exceeded by the requested amount.
    QuotaExceeded {
        resource: String,
        limit: u64,
        requested: u64,
    },
    /// A policy evaluation denied an action (either no rule allowed
    /// it, or an explicit deny matched).
    PolicyDenied {
        principal: String,
        action: String,
        resource: String,
    },
    /// A lookup by id found nothing.
    NotFound { what: &'static str, id: String },
    /// An operation conflicts with existing state (e.g. a duplicate
    /// resource id).
    Conflict { what: &'static str, id: String },
}

impl fmt::Display for CloudError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloudError::InvalidFormat {
                what,
                value,
                reason,
            } => {
                write!(f, "invalid {what} {value:?}: {reason}")
            }
            CloudError::InvalidTransition { what, from, to } => {
                write!(f, "invalid {what} transition: {from} -> {to}")
            }
            CloudError::QuotaExceeded {
                resource,
                limit,
                requested,
            } => write!(
                f,
                "quota exceeded for {resource}: requested {requested}, limit {limit}"
            ),
            CloudError::PolicyDenied {
                principal,
                action,
                resource,
            } => write!(f, "policy denied: {principal} may not {action} {resource}"),
            CloudError::NotFound { what, id } => write!(f, "{what} not found: {id}"),
            CloudError::Conflict { what, id } => write!(f, "{what} conflict: {id}"),
        }
    }
}

impl std::error::Error for CloudError {}

pub type CloudResult<T> = Result<T, CloudError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_format_display_includes_what_value_and_reason() {
        let e = CloudError::InvalidFormat {
            what: "account id",
            value: "abc".to_string(),
            reason: "must be 12 ASCII digits".to_string(),
        };
        let s = e.to_string();
        assert!(s.contains("account id"));
        assert!(s.contains("abc"));
        assert!(s.contains("12 ASCII digits"));
    }

    #[test]
    fn quota_exceeded_display_includes_all_three_numbers() {
        let e = CloudError::QuotaExceeded {
            resource: "instances".to_string(),
            limit: 5,
            requested: 6,
        };
        let s = e.to_string();
        assert!(s.contains("instances"));
        assert!(s.contains('5'));
        assert!(s.contains('6'));
    }

    #[test]
    fn policy_denied_display_reads_as_a_sentence() {
        let e = CloudError::PolicyDenied {
            principal: "user/alice".to_string(),
            action: "delete".to_string(),
            resource: "instance/i-1".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "policy denied: user/alice may not delete instance/i-1"
        );
    }

    #[test]
    fn errors_implement_std_error() {
        fn assert_is_error<E: std::error::Error>(_: &E) {}
        let e = CloudError::NotFound {
            what: "resource",
            id: "r-1".to_string(),
        };
        assert_is_error(&e);
    }

    #[test]
    fn equal_errors_compare_equal() {
        let a = CloudError::Conflict {
            what: "resource",
            id: "r-1".to_string(),
        };
        let b = CloudError::Conflict {
            what: "resource",
            id: "r-1".to_string(),
        };
        assert_eq!(a, b);
    }
}
