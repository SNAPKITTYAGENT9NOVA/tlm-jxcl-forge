// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Converts a [`cloud_policy::Policy`] to and from a JSON-shaped
//! document -- the third piece of Phase 13's IAM surface
//! (`cloud-identity`'s Phase 1 doc comment: "credentials, sessions,
//! federation, policy documents ... is Phase 13").
//!
//! The document shape:
//!
//! ```json
//! {
//!   "Statement": [
//!     {
//!       "Effect": "Allow",
//!       "Principal": "*",
//!       "Action": ["compute:describe"],
//!       "Resource": ["*"]
//!     }
//!   ]
//! }
//! ```
//!
//! [`from_json`] is a **hand-rolled parser restricted to exactly this
//! schema** -- strings, arrays, and objects only, no numbers, booleans,
//! or `null` -- rather than a pull of a general-purpose JSON crate,
//! keeping this workspace's zero-external-dependency posture intact
//! exactly as `cloud-checksum` (Phase 5) implements CRC-32 by hand and
//! `cloud-types::Arn` (Phase 1) hand-rolls its own `Display`/`FromStr`
//! pair rather than depending on a generic serialization framework.
//! Unlike AWS's own IAM policy JSON, `Principal`/`Action`/`Resource`
//! here are never polymorphic between a bare string and an array (the
//! one exception is the literal `"*"` for `Principal`, standing for
//! [`cloud_policy::PrincipalMatcher::Any`]) -- one shape per field,
//! not the either-or AWS's schema allows, exactly the kind of
//! deliberate scope-narrowing `cloud-policy`'s own `matches_pattern`
//! (a single trailing-wildcard position, not a general glob engine)
//! already applies elsewhere in this workspace.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_identity::Principal;
use cloud_policy::{Effect, Policy, PrincipalMatcher, Statement};

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn write_string_array(out: &mut String, indent: &str, items: &[String]) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push_str("[\n");
    for (i, item) in items.iter().enumerate() {
        out.push_str(indent);
        out.push_str("  \"");
        out.push_str(&escape(item));
        out.push('"');
        if i + 1 != items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(indent);
    out.push(']');
}

/// Serializes `policy` to a JSON-shaped document.
pub fn to_json(policy: &Policy) -> String {
    let statements = policy.statements();
    if statements.is_empty() {
        return "{\n  \"Statement\": []\n}".to_string();
    }

    let mut out = String::from("{\n  \"Statement\": [\n");
    for (i, statement) in statements.iter().enumerate() {
        out.push_str("    {\n");

        out.push_str("      \"Effect\": \"");
        out.push_str(match statement.effect {
            Effect::Allow => "Allow",
            Effect::Deny => "Deny",
        });
        out.push_str("\",\n");

        out.push_str("      \"Principal\": ");
        match &statement.principals {
            PrincipalMatcher::Any => out.push_str("\"*\""),
            PrincipalMatcher::OneOf(list) => {
                let strings: Vec<String> = list.iter().map(|p| p.to_string()).collect();
                write_string_array(&mut out, "      ", &strings);
            }
        }
        out.push_str(",\n");

        out.push_str("      \"Action\": ");
        write_string_array(&mut out, "      ", &statement.actions);
        out.push_str(",\n");

        out.push_str("      \"Resource\": ");
        write_string_array(&mut out, "      ", &statement.resources);
        out.push('\n');

        out.push_str("    }");
        if i + 1 != statements.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}");
    out
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Parser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn err(&self, reason: impl Into<String>) -> CloudError {
        let remaining: String = self.chars[self.pos..].iter().take(30).collect();
        CloudError::InvalidFormat {
            what: "policy document",
            value: remaining,
            reason: reason.into(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), CloudError> {
        self.skip_ws();
        match self.advance() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(self.err(format!("expected '{expected}', found '{c}'"))),
            None => Err(self.err(format!("expected '{expected}', found end of input"))),
        }
    }

    fn parse_string(&mut self) -> Result<String, CloudError> {
        self.skip_ws();
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(out),
                Some('\\') => match self.advance() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => return Err(self.err(format!("unsupported escape '\\{other}'"))),
                    None => return Err(self.err("unterminated escape at end of input")),
                },
                Some(c) => out.push(c),
                None => return Err(self.err("unterminated string")),
            }
        }
    }

    fn parse_string_array(&mut self) -> Result<Vec<String>, CloudError> {
        self.skip_ws();
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.advance();
            return Ok(items);
        }
        loop {
            items.push(self.parse_string()?);
            self.skip_ws();
            match self.advance() {
                Some(',') => continue,
                Some(']') => break,
                Some(c) => return Err(self.err(format!("expected ',' or ']', found '{c}'"))),
                None => return Err(self.err("unterminated array")),
            }
        }
        Ok(items)
    }

    /// A `"Principal"` value: either the literal string `"*"` or an
    /// array of `"kind/id"` principal strings.
    fn parse_principal_matcher(&mut self) -> Result<PrincipalMatcher, CloudError> {
        self.skip_ws();
        match self.peek() {
            Some('"') => {
                let s = self.parse_string()?;
                if s == "*" {
                    Ok(PrincipalMatcher::Any)
                } else {
                    Err(self.err(format!("Principal string must be \"*\", found {s:?}")))
                }
            }
            Some('[') => {
                let raw = self.parse_string_array()?;
                let principals = raw
                    .into_iter()
                    .map(|s| s.parse::<Principal>())
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(PrincipalMatcher::OneOf(principals))
            }
            Some(c) => Err(self.err(format!("expected Principal string or array, found '{c}'"))),
            None => Err(self.err("expected Principal, found end of input")),
        }
    }

    fn parse_effect(&mut self) -> Result<Effect, CloudError> {
        let s = self.parse_string()?;
        match s.as_str() {
            "Allow" => Ok(Effect::Allow),
            "Deny" => Ok(Effect::Deny),
            other => Err(self.err(format!(
                "Effect must be \"Allow\" or \"Deny\", found {other:?}"
            ))),
        }
    }

    /// One `{ "Effect": ..., "Principal": ..., "Action": ..., "Resource": ... }`
    /// object. Keys may appear in any order; all four are required.
    fn parse_statement(&mut self) -> Result<Statement, CloudError> {
        self.skip_ws();
        self.expect('{')?;
        let mut effect = None;
        let mut principals = None;
        let mut actions = None;
        let mut resources = None;

        self.skip_ws();
        if self.peek() == Some('}') {
            self.advance();
        } else {
            loop {
                let key = self.parse_string()?;
                self.expect(':')?;
                match key.as_str() {
                    "Effect" => effect = Some(self.parse_effect()?),
                    "Principal" => principals = Some(self.parse_principal_matcher()?),
                    "Action" => actions = Some(self.parse_string_array()?),
                    "Resource" => resources = Some(self.parse_string_array()?),
                    other => return Err(self.err(format!("unknown statement key {other:?}"))),
                }
                self.skip_ws();
                match self.advance() {
                    Some(',') => continue,
                    Some('}') => break,
                    Some(c) => return Err(self.err(format!("expected ',' or '}}', found '{c}'"))),
                    None => return Err(self.err("unterminated statement object")),
                }
            }
        }

        Ok(Statement {
            effect: effect.ok_or_else(|| self.err("statement missing \"Effect\""))?,
            principals: principals.ok_or_else(|| self.err("statement missing \"Principal\""))?,
            actions: actions.ok_or_else(|| self.err("statement missing \"Action\""))?,
            resources: resources.ok_or_else(|| self.err("statement missing \"Resource\""))?,
        })
    }

    fn parse_statement_array(&mut self) -> Result<Vec<Statement>, CloudError> {
        self.skip_ws();
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.advance();
            return Ok(items);
        }
        loop {
            items.push(self.parse_statement()?);
            self.skip_ws();
            match self.advance() {
                Some(',') => continue,
                Some(']') => break,
                Some(c) => return Err(self.err(format!("expected ',' or ']', found '{c}'"))),
                None => return Err(self.err("unterminated Statement array")),
            }
        }
        Ok(items)
    }

    fn parse_document(&mut self) -> Result<Policy, CloudError> {
        self.skip_ws();
        self.expect('{')?;
        let key = self.parse_string()?;
        if key != "Statement" {
            return Err(self.err(format!(
                "expected top-level key \"Statement\", found {key:?}"
            )));
        }
        self.expect(':')?;
        let statements = self.parse_statement_array()?;
        self.skip_ws();
        self.expect('}')?;
        self.skip_ws();
        if self.pos != self.chars.len() {
            return Err(self.err("unexpected trailing content after top-level object"));
        }

        let mut policy = Policy::new();
        for statement in statements {
            policy.add_statement(statement);
        }
        Ok(policy)
    }
}

/// Parses a JSON-shaped policy document back into a [`Policy`].
pub fn from_json(input: &str) -> Result<Policy, CloudError> {
    Parser::new(input).parse_document()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_policy::{Effect, PrincipalMatcher, Statement};
    use cloud_types::ResourceId;

    fn user(name: &str) -> Principal {
        Principal::User(ResourceId::new(name).unwrap())
    }

    #[test]
    fn an_empty_policy_round_trips() {
        let policy = Policy::new();
        let json = to_json(&policy);
        let parsed = from_json(&json).unwrap();
        assert_eq!(policy, parsed);
    }

    #[test]
    fn a_single_allow_all_statement_round_trips() {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::Any,
            actions: vec!["compute:describe".to_string()],
            resources: vec!["*".to_string()],
        });
        let json = to_json(&policy);
        let parsed = from_json(&json).unwrap();
        assert_eq!(policy, parsed);
    }

    #[test]
    fn multiple_statements_with_a_one_of_principal_round_trip() {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::Any,
            actions: vec!["*".to_string()],
            resources: vec!["*".to_string()],
        });
        policy.add_statement(Statement {
            effect: Effect::Deny,
            principals: PrincipalMatcher::OneOf(vec![user("alice"), user("bob")]),
            actions: vec![
                "compute:terminate".to_string(),
                "storage:delete-volume".to_string(),
            ],
            resources: vec!["i-1".to_string()],
        });
        let json = to_json(&policy);
        let parsed = from_json(&json).unwrap();
        assert_eq!(policy, parsed);
    }

    #[test]
    fn to_json_contains_the_expected_literal_fields() {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Deny,
            principals: PrincipalMatcher::Any,
            actions: vec!["compute:terminate".to_string()],
            resources: vec!["*".to_string()],
        });
        let json = to_json(&policy);
        assert!(json.contains("\"Effect\": \"Deny\""));
        assert!(json.contains("\"Principal\": \"*\""));
        assert!(json.contains("\"compute:terminate\""));
    }

    #[test]
    fn from_json_rejects_an_unknown_effect() {
        let err = from_json(
            r#"{"Statement": [{"Effect": "Maybe", "Principal": "*", "Action": ["*"], "Resource": ["*"]}]}"#,
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
    }

    #[test]
    fn from_json_rejects_a_statement_missing_a_required_key() {
        let err = from_json(
            r#"{"Statement": [{"Effect": "Allow", "Action": ["*"], "Resource": ["*"]}]}"#,
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
    }

    #[test]
    fn from_json_rejects_an_invalid_principal_string() {
        let err = from_json(
            r#"{"Statement": [{"Effect": "Allow", "Principal": ["robot/alice"], "Action": ["*"], "Resource": ["*"]}]}"#,
        )
        .unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
    }

    #[test]
    fn from_json_rejects_malformed_json() {
        let err = from_json(r#"{"Statement": [}"#).unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
    }

    #[test]
    fn from_json_rejects_trailing_content() {
        let err = from_json(r#"{"Statement": []}garbage"#).unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
    }

    #[test]
    fn from_json_accepts_an_explicitly_empty_statement_array() {
        let policy = from_json(r#"{"Statement": []}"#).unwrap();
        assert_eq!(policy.statements().len(), 0);
    }

    #[test]
    fn statement_keys_may_appear_in_any_order() {
        let policy = from_json(
            r#"{"Statement": [{"Resource": ["*"], "Action": ["*"], "Principal": "*", "Effect": "Allow"}]}"#,
        )
        .unwrap();
        assert_eq!(policy.statements()[0].effect, Effect::Allow);
    }
}
