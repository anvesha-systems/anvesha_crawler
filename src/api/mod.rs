pub mod auth;
pub mod handlers;

use std::sync::Arc;

use axum::{Router, middleware, routing::get};
use serde::Serialize;

use crate::SearchEngine;

// ─── Shared application state ─────────────────────────────────────────────────

pub struct AppState {
    pub engine: SearchEngine,
    /// If `Some`, all `/v1/*` routes require `Authorization: Bearer <key>`.
    /// Never logged.
    pub api_key: Option<String>,
}

// ─── Shared error body type ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

// ─── Router ───────────────────────────────────────────────────────────────────

/// Build the Axum router.
pub fn build_router(state: Arc<AppState>) -> Router {
    let v1 = Router::new().route("/search", get(handlers::search)).layer(
        middleware::from_fn_with_state(state.clone(), auth::require_auth),
    );

    Router::new()
        .route("/healthz", get(handlers::healthz))
        .nest("/v1", v1)
        .with_state(state)
}
