use std::sync::Arc;

use axum::extract::Request;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::api::{AppState, ErrorBody};

/// Middleware that enforces Bearer token auth when `AppState.api_key` is set.
///
/// If `api_key` is `None`, all requests pass through (local dev mode).
/// If `api_key` is `Some(key)`, the request must carry:
///   Authorization: Bearer \<key\>
///
/// The key value is never logged.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(ref expected) = state.api_key else {
        // No key configured — allow all requests.
        return next.run(request).await;
    };

    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                error: "missing_authorization_header".into(),
            }),
        )
            .into_response(),
        Some(header) => {
            let token = match header.strip_prefix("Bearer ") {
                Some(t) => t,
                None => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorBody {
                            error: "malformed_authorization_header".into(),
                        }),
                    )
                        .into_response();
                }
            };

            // Constant-time comparison to avoid timing-based key enumeration.
            if !constant_time_eq(token.as_bytes(), expected.as_bytes()) {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorBody {
                        error: "invalid_api_key".into(),
                    }),
                )
                    .into_response();
            }

            next.run(request).await
        }
    }
}

/// Constant-time byte comparison — avoids short-circuit leaking key length via timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
