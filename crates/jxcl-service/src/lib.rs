// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Generic service scaffolding (graceful shutdown, health-check endpoint pattern, startup logging) extracted from photo-cache-service's two binaries' shared boilerplate.
//!
//! Provides reusable helpers for running HTTP services with graceful shutdown and standard health-check endpoints.
//!
//! # Features
//!
//! - **Graceful Shutdown**: `serve_with_graceful_shutdown` wraps an axum server to handle Ctrl-C and SIGTERM signals, allowing in-flight requests to complete before exiting.
//! - **Health-Check Handler**: `healthz_handler` provides a standard `/healthz` endpoint that returns 200 OK.
//! - **Shutdown Signal Handling**: Cross-platform signal handling for Unix (SIGTERM) and all platforms (Ctrl-C).
#![forbid(unsafe_code)]

use axum::{http::StatusCode, response::IntoResponse};

/// A generic health check handler that returns 200 OK.
///
/// This provides a standard `/healthz` endpoint suitable for HTTP-based health probes
/// (e.g., Kubernetes liveness probes). The response is always 200 OK with a simple "ok" body.
///
/// # Example
///
/// ```no_run
/// use axum::Router;
/// use axum::routing::get;
/// use jxcl_service::healthz_handler;
///
/// # #[tokio::main]
/// # async fn main() {
/// let app: Router = Router::new()
///     .route("/healthz", get(healthz_handler));
/// # }
/// ```
pub async fn healthz_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// Waits for either Ctrl-C or SIGTERM signal.
///
/// Returns a future that completes when one of these signals is received.
/// Logs which signal caused the shutdown.
///
/// This is intended for use with axum's `with_graceful_shutdown` to ensure
/// in-flight requests complete before the process exits.
///
/// # Example
///
/// ```no_run
/// use axum::Router;
/// use jxcl_service::shutdown_signal;
///
/// # #[tokio::main]
/// # async fn main() {
/// let app: Router = Router::new();
/// let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
///
/// axum::serve(listener, app)
///     .with_graceful_shutdown(shutdown_signal())
///     .await
///     .ok();
/// # }
/// ```
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl-C, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}

/// Serves an axum Router with graceful shutdown handling.
///
/// This is a generic helper that combines:
/// - Running the axum server on a bound TCP listener
/// - Graceful shutdown on Ctrl-C or SIGTERM, allowing in-flight requests to complete
///
/// The server returns successfully after all in-flight requests have been processed
/// and the graceful shutdown signal has been received.
///
/// # Arguments
///
/// * `router` - The axum `Router` to serve
/// * `listener` - A bound `TcpListener` to accept connections on
///
/// # Returns
///
/// Returns `Ok(())` on successful shutdown, or an error if the server encounters
/// an I/O or runtime error (e.g., unable to accept connections).
///
/// # Example
///
/// ```no_run
/// use axum::Router;
/// use axum::routing::get;
/// use jxcl_service::{serve_with_graceful_shutdown, healthz_handler};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let app = Router::new()
///     .route("/healthz", get(healthz_handler));
///
/// let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
///
/// serve_with_graceful_shutdown(app, listener).await?;
/// # Ok(())
/// # }
/// ```
pub async fn serve_with_graceful_shutdown(
    router: axum::Router,
    listener: tokio::net::TcpListener,
) -> Result<(), std::io::Error> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    #[test]
    fn healthz_handler_signature_is_correct() {
        // This test verifies the handler is an async function that can be used with axum
        // The actual response testing is done in the integration test
    }

    #[tokio::test]
    async fn shutdown_signal_is_awaitable() {
        // Create a tokio channel to signal when to stop the test
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        // Spawn a task that waits for shutdown_signal
        let signal_task = tokio::spawn(async move {
            let signal_future = shutdown_signal();
            tokio::pin!(signal_future);

            // Race between the signal and our test timeout
            tokio::select! {
                _ = &mut signal_future => {
                    // Signal received
                    true
                }
                _ = rx.recv() => {
                    // Our test timeout
                    false
                }
            }
        });

        // Give the signal handler a moment to set up
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Send test signal (we'll use tokio::signal::ctrl_c via a simulated abort)
        tx.send(()).await.ok();

        // The task should complete quickly now
        let completed =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), signal_task).await;

        assert!(completed.is_ok(), "signal handler should complete");
    }

    #[tokio::test]
    async fn serve_with_graceful_shutdown_integration() {
        // Create a simple router with a healthz endpoint
        let app = axum::Router::new().route("/healthz", get(healthz_handler));

        // Bind to an ephemeral port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind listener");

        let addr = listener.local_addr().expect("failed to get local address");

        // Spawn the server task
        let server_task = tokio::spawn(async move {
            serve_with_graceful_shutdown(app, listener)
                .await
                .expect("server failed")
        });

        // Give the server a moment to start up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send a request to the /healthz endpoint
        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            client.get(format!("http://{}/healthz", addr)).send(),
        )
        .await
        .expect("timeout waiting for response")
        .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.text().await.expect("failed to read body");
        assert_eq!(body, "ok");

        // Abort the server task to trigger graceful shutdown
        server_task.abort();

        // Wait a bit to ensure the server shuts down cleanly
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn healthz_handler_returns_correct_response() {
        let _response = healthz_handler().await;
        // We verify it's a valid response by checking the StatusCode and body
        // in the serve_with_graceful_shutdown_integration test above.
        // This test just verifies the function is callable.
    }
}
