//! Shared plumbing for the `server` and `server-cached` binaries: both
//! are a faithful Rust port of the original `server.js`/`server-cached.js`
//! Express demo, hardened for production use (structured logging,
//! env-var configuration, timeouts, graceful shutdown, and no panics on
//! bad input or upstream failure).
#![forbid(unsafe_code)]

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::time::Duration;

pub const UPSTREAM_URL: &str = "https://jsonplaceholder.typicode.com/photos";

/// Build the shared `reqwest` client used to call the upstream API.
/// A bounded timeout is load-bearing: without it, a hung upstream
/// connection would hold a request (and, in the cached server, a Redis
/// connection-manager lock) open indefinitely.
pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client construction with only a timeout cannot fail")
}

/// Fetch the upstream photo list, returning it along with how long the
/// fetch took.
pub async fn fetch_photos(
    client: &reqwest::Client,
) -> Result<(Duration, Vec<serde_json::Value>), AppError> {
    let start = std::time::Instant::now();
    let response = client
        .get(UPSTREAM_URL)
        .send()
        .await
        .map_err(AppError::Upstream)?;
    if !response.status().is_success() {
        return Err(AppError::UpstreamStatus(response.status()));
    }
    let photos: Vec<serde_json::Value> = response.json().await.map_err(AppError::Upstream)?;
    Ok((start.elapsed(), photos))
}

#[derive(Debug, Serialize)]
pub struct PhotosResponse {
    pub cached: bool,
    pub duration_ms: u128,
    pub count: usize,
    pub sample: Vec<serde_json::Value>,
}

impl PhotosResponse {
    pub fn new(cached: bool, duration: Duration, photos: &[serde_json::Value]) -> Self {
        PhotosResponse {
            cached,
            duration_ms: duration.as_millis(),
            count: photos.len(),
            sample: photos.iter().take(3).cloned().collect(),
        }
    }
}

/// Application error type. Every variant maps to a specific HTTP status
/// so a failure — upstream API down, Redis unreachable — becomes a
/// well-formed error response, never a panic or a hung connection.
#[derive(Debug)]
pub enum AppError {
    Upstream(reqwest::Error),
    UpstreamStatus(reqwest::StatusCode),
    Cache(pq_cache::CacheError),
    Serialization(serde_json::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Upstream(e) => write!(f, "upstream request failed: {}", e),
            AppError::UpstreamStatus(s) => write!(f, "upstream returned status {}", s),
            AppError::Cache(e) => write!(f, "cache error: {}", e),
            AppError::Serialization(e) => write!(f, "serialization error: {}", e),
        }
    }
}
impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // The detailed error (which for `Cache`/`Upstream` can include
        // internal details like hostnames from a `redis`/`reqwest` error)
        // goes to the server's own logs only. Callers get a fixed,
        // generic message per status code -- mirroring `healthz`'s
        // "redis unavailable" pattern -- so an unauthenticated caller
        // can't use transient infra failures for reconnaissance.
        tracing::error!(error = %self, "request failed");
        let (status, message) = match &self {
            AppError::Upstream(_) | AppError::UpstreamStatus(_) => {
                (StatusCode::BAD_GATEWAY, "upstream request failed")
            }
            AppError::Cache(_) => (StatusCode::SERVICE_UNAVAILABLE, "cache unavailable"),
            AppError::Serialization(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

/// Initialize structured logging. Honors `RUST_LOG` (defaulting to
/// `info`) so operators can turn up verbosity without a redeploy.
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

/// Read `name` from the environment, or fall back to `default`.
///
/// Deliberately does *not* log the resolved value: some config (e.g.
/// `REDIS_URL`) can embed a credential (`redis://user:pass@host`), and a
/// generic helper is the wrong place to decide what's safe to print.
/// Callers log whatever they resolve, redacting first if it might be
/// sensitive (see [`redact_url_credentials`]).
pub fn env_or(name: &str, default: &str) -> String {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => v,
        _ => default.to_string(),
    }
}

pub fn env_or_u16(name: &str, default: u16) -> u16 {
    match std::env::var(name).ok().and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => default,
    }
}

pub fn env_or_u64(name: &str, default: u64) -> u64 {
    match std::env::var(name).ok().and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => default,
    }
}

/// True when `APP_ENV` names a production environment (case-insensitive
/// exact match on `"production"`). Anything else -- unset, `"dev"`,
/// `"staging"`, a typo -- is treated as non-production, so a
/// misconfigured deployment fails open to "allow plaintext Redis" rather
/// than failing closed and refusing to start; see
/// `check_redis_tls_requirement`'s doc comment for why that's the
/// intentional trade-off here.
pub fn is_production_env() -> bool {
    std::env::var("APP_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

/// In a production environment (`APP_ENV=production`), refuse to start
/// against a Redis URL that doesn't request TLS (`rediss://`). Outside
/// production this is a no-op: local development against a plaintext
/// `redis://127.0.0.1` is expected and fine (see `docs/HARDENING.md`
/// "Threat model" -- encrypting values with `pq-crypto` before they
/// reach Redis is not a substitute for transport security, so
/// production deployments must not skip it).
///
/// This only checks the URL scheme, not that the target actually speaks
/// TLS -- `redis::Client::open` will fail fast on that mismatch, and
/// this check exists to catch the *configuration* mistake of pointing a
/// production deployment at a plaintext endpoint in the first place.
pub fn check_redis_tls_requirement(redis_url: &str) -> Result<(), String> {
    check_redis_tls_requirement_for(redis_url, is_production_env())
}

/// The actual (pure, env-independent) check behind
/// [`check_redis_tls_requirement`], split out so it's testable without
/// mutating process-global environment variables (which would race
/// against other tests running in parallel in the same process).
fn check_redis_tls_requirement_for(redis_url: &str, is_production: bool) -> Result<(), String> {
    if is_production && !redis_url.starts_with("rediss://") {
        return Err(
            "APP_ENV=production requires REDIS_URL to use the rediss:// (TLS) scheme, \
             but a non-TLS URL was given. Use rediss:// (with a TLS-capable Redis), \
             or unset APP_ENV for local/non-production use."
                .to_string(),
        );
    }
    Ok(())
}

/// Replace any `user:password@` userinfo in a URL with `***@` before
/// it's logged. Best-effort string surgery (not a full URL parser) is
/// deliberate here: we never want a parse failure to fall back to
/// printing the original, credential-bearing string.
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

/// Load a 64-byte ML-KEM seed from the hex-encoded `PQ_KEM_SEED`
/// environment variable, for deriving the same key pair across multiple
/// service replicas (see `docs/HARDENING.md`). Returns `None` (and logs
/// a warning explaining the consequence) if it's unset.
pub fn keypair_from_env() -> pq_crypto::KeyPair {
    match std::env::var("PQ_KEM_SEED") {
        Ok(hex) if !hex.is_empty() => match decode_hex_seed(&hex) {
            Some(seed) => {
                tracing::info!("PQ_KEM_SEED set: deriving a stable key pair (shared across replicas using the same seed)");
                pq_crypto::KeyPair::from_seed(&seed)
            }
            None => {
                tracing::error!(
                    "PQ_KEM_SEED is set but is not {} hex characters ({} bytes); refusing to start with an ambiguous key configuration",
                    pq_crypto::SEED_LEN * 2,
                    pq_crypto::SEED_LEN
                );
                std::process::exit(1);
            }
        },
        _ => {
            tracing::warn!(
                "PQ_KEM_SEED not set: generating a random key pair for this process only. \
                 Cache entries will NOT be readable by other replicas or after a restart \
                 (safe, but defeats sharing a cache across instances -- see docs/HARDENING.md)."
            );
            pq_crypto::KeyPair::generate()
        }
    }
}

fn decode_hex_seed(hex: &str) -> Option<[u8; pq_crypto::SEED_LEN]> {
    if hex.len() != pq_crypto::SEED_LEN * 2 {
        return None;
    }
    let mut seed = [0u8; pq_crypto::SEED_LEN];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(seed)
}

/// Waits for either Ctrl-C or SIGTERM, for use with axum's
/// `with_graceful_shutdown` so in-flight requests finish before the
/// process exits (important under a process supervisor / container
/// orchestrator that sends SIGTERM before SIGKILL).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password_from_redis_url() {
        assert_eq!(
            redact_url_credentials("redis://:supersecret@localhost:6379"),
            "redis://***@localhost:6379"
        );
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
    fn malformed_url_is_returned_as_is_rather_than_guessed_at() {
        assert_eq!(
            redact_url_credentials("not-a-url-at-all"),
            "not-a-url-at-all"
        );
    }

    #[test]
    fn decode_hex_seed_accepts_exact_length_hex() {
        let hex = "ab".repeat(pq_crypto::SEED_LEN);
        let seed = decode_hex_seed(&hex).unwrap();
        assert_eq!(seed, [0xabu8; pq_crypto::SEED_LEN]);
    }

    #[test]
    fn decode_hex_seed_rejects_wrong_length_or_bad_hex() {
        assert!(decode_hex_seed("ab").is_none());
        assert!(decode_hex_seed(&"zz".repeat(pq_crypto::SEED_LEN)).is_none());
    }

    #[test]
    fn production_requires_tls_redis_url() {
        assert!(check_redis_tls_requirement_for("redis://127.0.0.1:6379", true).is_err());
        assert!(check_redis_tls_requirement_for("rediss://127.0.0.1:6379", true).is_ok());
    }

    #[test]
    fn non_production_allows_plaintext_redis_url() {
        assert!(check_redis_tls_requirement_for("redis://127.0.0.1:6379", false).is_ok());
        assert!(check_redis_tls_requirement_for("rediss://127.0.0.1:6379", false).is_ok());
    }
}
