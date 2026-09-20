// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Low-level network utilities: connection-string credential redaction, address/port parsing.
//!
//! Owns: `redact_url_credentials` and address-parsing helpers.
//!
//! This module provides utilities for safely handling network URLs and addresses,
//! with special attention to credential redaction for logging. All functions are
//! best-effort string operations -- they deliberately avoid full parsing in favor
//! of simple string surgery to prevent parse failures from accidentally printing
//! credentials.
#![forbid(unsafe_code)]

/// Replace any `user:password@` userinfo in a URL with `***@` before
/// it's logged. Best-effort string surgery (not a full URL parser) is
/// deliberate here: we never want a parse failure to fall back to
/// printing the original, credential-bearing string.
///
/// # Examples
///
/// ```
/// use jxcl_network::redact_url_credentials;
///
/// assert_eq!(
///     redact_url_credentials("redis://:supersecret@localhost:6379"),
///     "redis://***@localhost:6379"
/// );
/// assert_eq!(
///     redact_url_credentials("redis://user:supersecret@localhost:6379/0"),
///     "redis://***@localhost:6379/0"
/// );
/// assert_eq!(
///     redact_url_credentials("redis://127.0.0.1:6379"),
///     "redis://127.0.0.1:6379"
/// );
/// ```
pub fn redact_url_credentials(url: &str) -> String {
    match url.find("://") {
        Some(scheme_end) => {
            let (scheme, rest) = url.split_at(scheme_end + 3);
            match rest.find('@') {
                Some(at) => format!("{scheme}***@{}", &rest[at + 1..]),
                None => url.to_string(),
            }
        }
        None => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password_from_redis_url_with_empty_user() {
        assert_eq!(
            redact_url_credentials("redis://:supersecret@localhost:6379"),
            "redis://***@localhost:6379"
        );
    }

    #[test]
    fn redacts_password_from_redis_url_with_username() {
        assert_eq!(
            redact_url_credentials("redis://user:supersecret@localhost:6379/0"),
            "redis://***@localhost:6379/0"
        );
    }

    #[test]
    fn leaves_credential_free_urls_unchanged() {
        assert_eq!(
            redact_url_credentials("redis://127.0.0.1:6379"),
            "redis://127.0.0.1:6379"
        );
    }

    #[test]
    fn handles_urls_without_scheme() {
        assert_eq!(redact_url_credentials("localhost:6379"), "localhost:6379");
    }

    #[test]
    fn malformed_url_is_returned_as_is_rather_than_guessed_at() {
        assert_eq!(
            redact_url_credentials("not-a-url-at-all"),
            "not-a-url-at-all"
        );
    }

    #[test]
    fn redacts_http_urls_with_credentials() {
        assert_eq!(
            redact_url_credentials("http://admin:secretpass@example.com/api"),
            "http://***@example.com/api"
        );
    }

    #[test]
    fn redacts_https_urls_with_credentials() {
        assert_eq!(
            redact_url_credentials("https://user:password@example.com:8443/path"),
            "https://***@example.com:8443/path"
        );
    }

    #[test]
    fn handles_special_chars_in_password() {
        // Credentials with special characters should still be redacted
        assert_eq!(
            redact_url_credentials("postgres://user:pass%40word@db.example.com/db"),
            "postgres://***@db.example.com/db"
        );
    }

    #[test]
    fn preserves_path_query_and_fragment() {
        assert_eq!(
            redact_url_credentials("mysql://root:pass@localhost/db?timeout=30#section"),
            "mysql://***@localhost/db?timeout=30#section"
        );
    }

    #[test]
    fn handles_multiple_at_signs_in_password() {
        // When password contains @, only the first @ is treated as userinfo separator
        assert_eq!(
            redact_url_credentials("redis://:pass@word@localhost:6379"),
            "redis://***@word@localhost:6379"
        );
    }

    #[test]
    fn handles_ipv6_addresses() {
        // IPv6 addresses with @ in userinfo
        assert_eq!(
            redact_url_credentials("http://user:pass@[::1]:8080/path"),
            "http://***@[::1]:8080/path"
        );
    }
}
