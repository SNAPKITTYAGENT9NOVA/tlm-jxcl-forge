// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Rust port of the original `server.js`: an uncached `/photos` endpoint
//! that fetches the upstream API on every request.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use photo_cache_service::{
    build_http_client, env_or_u16, fetch_photos, init_tracing, shutdown_signal, AppError,
    PhotosResponse,
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    http: reqwest::Client,
}

async fn photos_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PhotosResponse>, AppError> {
    let (duration, photos) = fetch_photos(&state.http).await?;
    tracing::info!(
        elapsed_ms = duration.as_millis(),
        count = photos.len(),
        "fetched photos (uncached)"
    );
    Ok(Json(PhotosResponse::new(false, duration, &photos)))
}

async fn healthz() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    init_tracing();

    let port = env_or_u16("PORT", 3000);
    let state = Arc::new(AppState {
        http: build_http_client(),
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
    tracing::info!(%addr, "server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}
