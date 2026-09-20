# jxcl-service

Generic service scaffolding (graceful shutdown, health-check endpoint pattern, startup logging) extracted from photo-cache-service's two binaries' shared boilerplate.

## Purpose

This crate provides reusable helpers for building HTTP services that:

- Handle graceful shutdown on Ctrl-C and SIGTERM signals, allowing in-flight requests to complete before exiting
- Provide a standard `/healthz` health-check endpoint suitable for HTTP-based probes (e.g., Kubernetes liveness/readiness checks)
- Implement cross-platform signal handling (Ctrl-C on all platforms, SIGTERM on Unix)

## Public API

### `healthz_handler()`

An async handler function that returns HTTP 200 OK with a simple "ok" body. Suitable for use as an axum route handler.

```rust
use axum::Router;
use axum::routing::get;
use jxcl_service::healthz_handler;

let app = Router::new()
    .route("/healthz", get(healthz_handler));
```

### `shutdown_signal()`

An async function that waits for either a Ctrl-C signal or SIGTERM (on Unix systems). Returns a future that completes when one of these signals is received. Designed for use with axum's `with_graceful_shutdown`.

```rust
use axum::Router;
use jxcl_service::shutdown_signal;

let app = Router::new();
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

### `serve_with_graceful_shutdown()`

A generic helper that combines running an axum server with automatic graceful shutdown. Takes a Router and a TcpListener, and returns a future that completes after the server shuts down gracefully.

```rust
use axum::Router;
use axum::routing::get;
use jxcl_service::{serve_with_graceful_shutdown, healthz_handler};

let app = Router::new()
    .route("/healthz", get(healthz_handler));

let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
serve_with_graceful_shutdown(app, listener).await?;
```

## Implementation Notes

- **Signal Handling**: The `shutdown_signal()` function uses `tokio::signal::ctrl_c()` for Ctrl-C handling and `tokio::signal::unix::signal()` for SIGTERM on Unix. On non-Unix platforms, only Ctrl-C is supported.
- **Graceful Shutdown**: In-flight requests are allowed to complete before the process exits, using axum's built-in `with_graceful_shutdown` mechanism.
- **Health Check Simplicity**: The `healthz_handler` intentionally returns a minimal response (200 OK) without checking dependencies. For more complex health checks that verify upstream services, callers should wrap or replace this handler.

## Testing

The crate includes 4 unit tests and integration tests:

- `healthz_handler_signature_is_correct`: Verifies the handler function signature
- `healthz_handler_returns_correct_response`: Tests the handler invocation
- `shutdown_signal_is_awaitable`: Verifies the shutdown signal can be awaited
- `serve_with_graceful_shutdown_integration`: Full integration test that starts a real axum server on an ephemeral port, sends an HTTP request to `/healthz`, verifies the response, and tests graceful shutdown

All tests pass with the crate's dependencies (axum, tokio, tracing, reqwest for tests).

## Dependencies

Workspace crates:
- `jxcl-http`
- `jxcl-logging`
- `jxcl-config`

External crates:
- `axum` - HTTP server framework
- `tokio` - Async runtime and signal handling
- `tracing` - Structured logging

Dev dependencies:
- `reqwest` - HTTP client for tests

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
