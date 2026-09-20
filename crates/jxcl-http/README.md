# jxcl-http

Shared HTTP client/server helpers (timeout wrapper, error-to-status mapping) extracted from photo-cache-service's duplicated per-binary logic.

## Purpose

Provides generic utilities for async timeout handling and error-to-HTTP-status mapping, designed to reduce code duplication across services that need consistent timeout and error handling patterns.

## Public API

- `with_timeout` - Wraps a future with a timeout, returning `TimeoutError` if the operation exceeds the specified duration.
- `error_to_status` - Maps `jxcl-errors::Error` types to HTTP status codes for consistent error responses.
- `TimeoutError` - Error type indicating an async operation timed out.

## Implementation Notes

The `with_timeout` function provides a generic wrapper around `tokio::time::timeout`, making it easy to apply timeouts to any async operation. This is particularly useful for operations that might block indefinitely, such as external API calls or database queries.

The `error_to_status` function maps the three variants of `jxcl-errors::Error` to appropriate HTTP status codes:
- `OutOfBounds` → 400 Bad Request
- `InvalidConfig` → 500 Internal Server Error
- `InvalidInstruction` → 400 Bad Request

## Testing

Implemented unit tests covering:
1. Timeout success case - futures completing within the timeout duration
2. Timeout failure case - futures exceeding the timeout duration
3. Error-to-status mapping for each `jxcl-errors::Error` variant (3 tests)

Total: 6 unit tests

## Dependencies

Workspace crates:
- `jxcl-errors`

External crates:
- `axum` 0.8 (for HTTP status codes)
- `reqwest` 0.13 (optional, for HTTP client context)
- `tokio` 1.x (for async runtime and timeout utilities)

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
