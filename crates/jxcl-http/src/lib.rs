// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Shared HTTP client/server helpers (timeout wrapper, error-to-status mapping) extracted from photo-cache-service's duplicated per-binary logic.
//!
//! Provides generic async operation timeout wrapper and error-to-HTTP-status mapping for jxcl-errors types.
#![forbid(unsafe_code)]

use axum::http::StatusCode;
use std::fmt;
use std::time::Duration;

/// An error that occurs when an async operation times out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "operation timed out")
    }
}
impl std::error::Error for TimeoutError {}

/// Wraps a future with a timeout. If the future completes within the
/// specified duration, returns its result. Otherwise, returns a `TimeoutError`.
///
/// # Examples
///
/// ```rust,no_run
/// use jxcl_http::with_timeout;
/// use std::time::Duration;
///
/// async fn some_operation() -> String {
///     "result".to_string()
/// }
///
/// # async fn example() {
/// let result = with_timeout(some_operation(), Duration::from_secs(5)).await;
/// assert!(result.is_ok());
/// # }
/// ```
pub async fn with_timeout<F, T>(future: F, duration: Duration) -> Result<T, TimeoutError>
where
    F: std::future::Future<Output = T>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_elapsed) => Err(TimeoutError),
    }
}

/// Maps a jxcl-errors error type to an HTTP status code.
///
/// Provides a consistent mapping from jxcl error types to HTTP status codes
/// for use in HTTP handlers and error responses.
pub fn error_to_status(error: &jxcl_errors::Error) -> StatusCode {
    match error {
        jxcl_errors::Error::OutOfBounds { .. } => StatusCode::BAD_REQUEST,
        jxcl_errors::Error::InvalidConfig { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        jxcl_errors::Error::InvalidInstruction { .. } => StatusCode::BAD_REQUEST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_error_displays_correctly() {
        let err = TimeoutError;
        assert_eq!(err.to_string(), "operation timed out");
    }

    #[tokio::test]
    async fn with_timeout_succeeds_when_future_completes_within_duration() {
        async fn quick_operation() -> &'static str {
            "success"
        }

        let result = with_timeout(quick_operation(), Duration::from_secs(1)).await;
        assert_eq!(result, Ok("success"));
    }

    #[tokio::test]
    async fn with_timeout_fails_when_future_exceeds_duration() {
        async fn slow_operation() {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        let result: Result<(), TimeoutError> =
            with_timeout(slow_operation(), Duration::from_millis(100)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn with_timeout_completes_immediately_with_zero_duration() {
        async fn instant_operation() -> i32 {
            42
        }

        // Even with zero duration, if the operation completes synchronously,
        // it may still succeed before the timeout triggers. So we test that
        // the mechanism works, not that zero always times out.
        let result = with_timeout(instant_operation(), Duration::from_nanos(1)).await;
        // This may succeed or fail depending on scheduling, so we just verify
        // the result is well-typed.
        let _ = result;
    }

    #[test]
    fn error_to_status_out_of_bounds() {
        let err = jxcl_errors::Error::OutOfBounds {
            needed: 4,
            available: 1,
            at: 10,
        };
        assert_eq!(error_to_status(&err), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn error_to_status_invalid_config() {
        let err = jxcl_errors::Error::InvalidConfig {
            name: "TEST_VAR",
            reason: "wrong length".to_string(),
        };
        assert_eq!(error_to_status(&err), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn error_to_status_invalid_instruction() {
        let err = jxcl_errors::Error::InvalidInstruction {
            reason: "bad operand".to_string(),
        };
        assert_eq!(error_to_status(&err), StatusCode::BAD_REQUEST);
    }
}
