//! Rust port of the original `server-cached.js`: a `/photos` endpoint
//! backed by a Redis cache whose entries are sealed with post-quantum
//! envelope encryption (`pq-crypto`/`pq-cache`) before ever reaching
//! Redis, demonstrating cache-hit vs. cache-miss latency exactly like
//! the original.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use photo_cache_service::{
    build_http_client, check_redis_tls_requirement, env_or, env_or_u16, env_or_u64, fetch_photos,
    init_tracing, keypair_from_env, redact_url_credentials, shutdown_signal, AppError,
    PhotosResponse,
};
use pq_cache::EncryptedCache;
use std::sync::Arc;
use tokio::sync::Mutex;

const CACHE_KEY: &str = "photos";

struct AppState {
    http: reqwest::Client,
    cache: Mutex<EncryptedCache>,
    ttl_seconds: u64,
}

async fn photos_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PhotosResponse>, AppError> {
    let start = std::time::Instant::now();

    let cached_bytes = {
        let mut cache = state.cache.lock().await;
        cache.get(CACHE_KEY).await.map_err(AppError::Cache)?
    };

    if let Some(bytes) = cached_bytes {
        match serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) {
            Ok(photos) => {
                let elapsed = start.elapsed();
                tracing::info!(elapsed_ms = elapsed.as_millis(), "cache HIT");
                return Ok(Json(PhotosResponse::new(true, elapsed, &photos)));
            }
            Err(e) => {
                // Shouldn't happen for entries we wrote ourselves, but a
                // corrupted cache entry must degrade to a miss, not a panic.
                tracing::warn!(error = %e, "cached payload was not valid JSON; treating as miss");
            }
        }
    }

    let (_fetch_duration, photos) = fetch_photos(&state.http).await?;
    let serialized = serde_json::to_vec(&photos).map_err(AppError::Serialization)?;
    {
        let mut cache = state.cache.lock().await;
        cache
            .set_with_ttl(CACHE_KEY, &serialized, state.ttl_seconds)
            .await
            .map_err(AppError::Cache)?;
    }

    let elapsed = start.elapsed();
    tracing::info!(
        elapsed_ms = elapsed.as_millis(),
        count = photos.len(),
        "cache MISS (fetched + stored)"
    );
    Ok(Json(PhotosResponse::new(false, elapsed, &photos)))
}

async fn healthz(State(state): State<Arc<AppState>>) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    let mut cache = state.cache.lock().await;
    match cache.ping().await {
        Ok(()) => (StatusCode::OK, "ok").into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "healthz: redis ping failed");
            (StatusCode::SERVICE_UNAVAILABLE, "redis unavailable").into_response()
        }
    }
}

#[tokio::main]
async fn main() {
    init_tracing();

    let port = env_or_u16("PORT", 3001);
    let redis_url = env_or("REDIS_URL", "redis://127.0.0.1:6379");
    let ttl_seconds = env_or_u64("CACHE_TTL_SECONDS", 3600);
    tracing::info!(port, redis_url = %redact_url_credentials(&redis_url), ttl_seconds, "config resolved");

    if let Err(e) = check_redis_tls_requirement(&redis_url) {
        tracing::error!(error = %e, "refusing to start");
        std::process::exit(1);
    }

    let keypair = keypair_from_env();
    let cache = match EncryptedCache::connect(&redis_url, keypair).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to redis at startup");
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState {
        http: build_http_client(),
        cache: Mutex::new(cache),
        ttl_seconds,
    });

    let app = Router::new()
        .route("/photos", get(photos_handler))
        .route("/healthz", get(healthz))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(%addr, error = %e, "failed to bind");
            std::process::exit(1);
        });
    tracing::info!(%addr, "cached server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}
