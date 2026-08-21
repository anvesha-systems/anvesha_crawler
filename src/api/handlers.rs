use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::api::AppState;
use crate::search::filters::{SearchFilter, SortBy};

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchItem>,
    pub took_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct SearchItem {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

pub async fn healthz() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    // Validate q
    let query = match params.q.as_deref() {
        None | Some("") => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "missing_or_empty_query" })),
            )
                .into_response();
        }
        Some(q) => q,
    };

    // Validate limit
    let limit = match params.limit {
        None => 10,
        Some(0) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "limit_must_be_at_least_1" })),
            )
                .into_response();
        }
        Some(n) if n > 100 => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "limit_exceeds_maximum_of_100" })),
            )
                .into_response();
        }
        Some(n) => n,
    };

    let offset = params.offset.unwrap_or(0);

    let start = Instant::now();

    let raw_results = state.engine.search(
        query,
        limit,
        offset,
        SearchFilter::new(),
        SortBy::Relevance,
        true,  // snippets
        false, // highlight
    );

    let took_ms = start.elapsed().as_millis() as u64;

    match raw_results {
        Ok(items) => {
            let results: Vec<SearchItem> = items
                .into_iter()
                .map(|r| SearchItem {
                    url: r.url,
                    title: r.title.unwrap_or_default(),
                    snippet: r.snippet.unwrap_or_default(),
                    score: r.score,
                })
                .collect();

            info!(
                result_count = results.len(),
                took_ms = took_ms,
                "search request completed"
            );

            (
                StatusCode::OK,
                Json(serde_json::to_value(SearchResponse { results, took_ms }).unwrap()),
            )
                .into_response()
        }
        Err(e) => {
            let category = classify_error(&e.to_string());
            warn!(error_category = %category, "search engine error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": category })),
            )
                .into_response()
        }
    }
}

/// Map internal errors to stable, opaque category strings.
/// Never returns raw error messages that might contain paths, queries, or internal state.
fn classify_error(msg: &str) -> &'static str {
    let lower = msg.to_lowercase();
    if lower.contains("index") || lower.contains("directory") || lower.contains("no such file") {
        "index_unavailable"
    } else if lower.contains("parse") || lower.contains("query") {
        "invalid_query"
    } else {
        "search_error"
    }
}
