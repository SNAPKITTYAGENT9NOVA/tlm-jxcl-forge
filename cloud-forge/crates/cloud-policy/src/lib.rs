// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! An IAM-style Allow/Deny policy evaluation engine.
//!
//! ## The dominant invariant
//!
//! From the roadmap: `principal + action + resource + condition +
//! context = ALLOW / DENY`, and **explicit deny must dominate**. This
//! crate implements exactly that: [`Policy::evaluate`] short-circuits
//! the instant any matching statement has [`Effect::Deny`], regardless
//! of how many matching `Allow` statements exist or in what order the
//! statements were added. With no matching statement at all, the
//! result is deny by default -- there is no implicit allow.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_identity::Principal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// Which principals a [`Statement`] applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalMatcher {
    /// Applies to every principal.
    Any,
    /// Applies only to the listed principals.
    OneOf(Vec<Principal>),
}

impl PrincipalMatcher {
    fn matches(&self, principal: &Principal) -> bool {
        match self {
            PrincipalMatcher::Any => true,
            PrincipalMatcher::OneOf(list) => list.contains(principal),
        }
    }
}

/// Matches `value` against `pattern`, where a trailing `*` in
/// `pattern` is a prefix wildcard (`"compute:*"` matches
/// `"compute:describe"`) and a bare `"*"` matches everything.
/// Anything else must match `value` exactly. This is deliberately not
/// a general glob engine -- one wildcard position, at the end, is all
/// any statement in this phase needs.
fn matches_pattern(pattern: &str, value: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => value.starts_with(prefix),
        None => pattern == value,
    }
}

/// One rule in a [`Policy`]: an effect that applies when a principal,
/// action, and resource all match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub effect: Effect,
    pub principals: PrincipalMatcher,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
}

impl Statement {
    fn matches(&self, principal: &Principal, action: &str, resource: &str) -> bool {
        self.principals.matches(principal)
            && self.actions.iter().any(|p| matches_pattern(p, action))
            && self.resources.iter().any(|p| matches_pattern(p, resource))
    }
}

/// An ordered set of [`Statement`]s. Order does not affect the
/// decision -- see the dominant invariant above -- but statements are
/// still stored in the order they were added for auditability.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    statements: Vec<Statement>,
}

impl Policy {
    pub fn new() -> Self {
        Policy {
            statements: Vec::new(),
        }
    }

    pub fn add_statement(&mut self, statement: Statement) -> &mut Self {
        self.statements.push(statement);
        self
    }

    /// This policy's statements, in the order they were added.
    ///
    /// Added for Phase 13's `cloud-policy-document`, which needs to
    /// serialize a `Policy` to JSON -- exactly the same reason
    /// `cloud-scheduler` (Phase 2) gained `place_least_loaded_with_capacity`
    /// in Phase 4 rather than that logic living somewhere else: a later
    /// phase's composition need is satisfied by adding a narrow,
    /// backward-compatible accessor to the crate that already owns the
    /// data, not by duplicating it.
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    /// Evaluates every statement against `(principal, action,
    /// resource)`. Any matching `Deny` wins immediately; otherwise the
    /// result is `Allow` only if at least one statement matched with
    /// `Allow`.
    pub fn evaluate(&self, principal: &Principal, action: &str, resource: &str) -> Decision {
        let mut allowed = false;
        for statement in &self.statements {
            if statement.matches(principal, action, resource) {
                match statement.effect {
                    Effect::Deny => return Decision::Deny,
                    Effect::Allow => allowed = true,
                }
            }
        }
        if allowed {
            Decision::Allow
        } else {
            Decision::Deny
        }
    }

    /// [`Self::evaluate`], turned into a `Result` for callers that
    /// want `?`-propagation on denial.
    pub fn authorize(
        &self,
        principal: &Principal,
        action: &str,
        resource: &str,
    ) -> Result<(), CloudError> {
        match self.evaluate(principal, action, resource) {
            Decision::Allow => Ok(()),
            Decision::Deny => Err(CloudError::PolicyDenied {
                principal: principal.to_string(),
                action: action.to_string(),
                resource: resource.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_identity::Principal;

    fn user(name: &str) -> Principal {
        Principal::User(cloud_types::ResourceId::new(name).unwrap())
    }

    fn allow_all(actions: &[&str], resources: &[&str]) -> Statement {
        Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::Any,
            actions: actions.iter().map(|s| s.to_string()).collect(),
            resources: resources.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn deny_all(actions: &[&str], resources: &[&str]) -> Statement {
        Statement {
            effect: Effect::Deny,
            principals: PrincipalMatcher::Any,
            actions: actions.iter().map(|s| s.to_string()).collect(),
            resources: resources.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn no_matching_statement_is_deny_by_default() {
        let policy = Policy::new();
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:describe", "i-1"),
            Decision::Deny
        );
    }

    #[test]
    fn a_single_matching_allow_is_allowed() {
        let mut policy = Policy::new();
        policy.add_statement(allow_all(&["compute:describe"], &["*"]));
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:describe", "i-1"),
            Decision::Allow
        );
    }

    #[test]
    fn explicit_deny_dominates_when_deny_is_added_after_allow() {
        let mut policy = Policy::new();
        policy.add_statement(allow_all(&["*"], &["*"]));
        policy.add_statement(deny_all(&["compute:terminate"], &["*"]));
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:terminate", "i-1"),
            Decision::Deny
        );
        // An unrelated action is still allowed by the first statement.
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:describe", "i-1"),
            Decision::Allow
        );
    }

    #[test]
    fn explicit_deny_dominates_when_deny_is_added_before_allow() {
        let mut policy = Policy::new();
        policy.add_statement(deny_all(&["compute:terminate"], &["*"]));
        policy.add_statement(allow_all(&["*"], &["*"]));
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:terminate", "i-1"),
            Decision::Deny
        );
    }

    #[test]
    fn wildcard_action_prefix_matches() {
        let mut policy = Policy::new();
        policy.add_statement(allow_all(&["compute:*"], &["*"]));
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:describe", "i-1"),
            Decision::Allow
        );
        assert_eq!(
            policy.evaluate(&user("alice"), "storage:get", "i-1"),
            Decision::Deny
        );
    }

    #[test]
    fn principal_matcher_one_of_restricts_to_listed_principals() {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::OneOf(vec![user("alice")]),
            actions: vec!["*".to_string()],
            resources: vec!["*".to_string()],
        });
        assert_eq!(
            policy.evaluate(&user("alice"), "compute:describe", "i-1"),
            Decision::Allow
        );
        assert_eq!(
            policy.evaluate(&user("bob"), "compute:describe", "i-1"),
            Decision::Deny
        );
    }

    #[test]
    fn authorize_returns_ok_on_allow_and_a_descriptive_err_on_deny() {
        let mut policy = Policy::new();
        policy.add_statement(allow_all(&["compute:describe"], &["*"]));

        assert!(policy
            .authorize(&user("alice"), "compute:describe", "i-1")
            .is_ok());

        let err = policy
            .authorize(&user("alice"), "compute:terminate", "i-1")
            .unwrap_err();
        match err {
            CloudError::PolicyDenied {
                principal,
                action,
                resource,
            } => {
                assert_eq!(principal, "user/alice");
                assert_eq!(action, "compute:terminate");
                assert_eq!(resource, "i-1");
            }
            other => panic!("expected PolicyDenied, got {other:?}"),
        }
    }

    #[test]
    fn statements_returns_them_in_the_order_they_were_added() {
        let mut policy = Policy::new();
        policy.add_statement(allow_all(&["a"], &["*"]));
        policy.add_statement(deny_all(&["b"], &["*"]));
        let statements = policy.statements();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].effect, Effect::Allow);
        assert_eq!(statements[1].effect, Effect::Deny);
    }

    #[test]
    fn matches_pattern_bare_star_matches_anything() {
        assert!(matches_pattern("*", "literally anything"));
        assert!(matches_pattern("*", ""));
    }

    #[test]
    fn matches_pattern_requires_exact_match_without_a_wildcard() {
        assert!(matches_pattern("compute:describe", "compute:describe"));
        assert!(!matches_pattern("compute:describe", "compute:describe-x"));
    }
}
