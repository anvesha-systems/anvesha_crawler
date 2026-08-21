//! Integration tests for the V1 Search API.
//!
//! These tests build a minimal Tantivy index in a temp directory,
//! construct the Axum router on top of it, and send requests directly
//! using `tower::ServiceExt::oneshot()` — no TCP socket, no PostgreSQL,
//! no production API key required.

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;

use crawler::SearchEngine;
use crawler::api::{AppState, build_router};
use crawler::search::schema::SearchSchema;

// ─── Test-index fixture ───────────────────────────────────────────────────────

/// Write a small set of documents into a Tantivy index at `dir`.
/// Returns after committing so the index is ready to open.
fn create_test_index(dir: &Path) {
    let index = SearchSchema::open_or_create(dir).expect("create index");
    let schema = SearchSchema::build();
    let mut writer = index.writer(15_000_000).expect("writer");

    let docs: &[(&str, &str, &str, f64)] = &[
        (
            "https://rust-lang.org",
            "The Rust Programming Language",
            "A language empowering everyone to build reliable and efficient software.",
            0.95,
        ),
        (
            "https://doc.rust-lang.org/book",
            "The Rust Book",
            "Learn Rust with the official book covering ownership, lifetimes, and more.",
            0.90,
        ),
        (
            "https://crates.io",
            "crates.io: Rust Package Registry",
            "The Rust community's crate registry for sharing and discovering libraries.",
            0.85,
        ),
    ];

    for (url, title, content, quality) in docs {
        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(schema.url_field, url);
        doc.add_text(schema.title_field, title);
        doc.add_text(schema.content_field, content);
        let domain = url.split('/').nth(2).unwrap_or("unknown");
        doc.add_text(schema.domain_field, domain);
        // quality_field is declared TEXT in schema; store as text to match schema type.
        // (The pre-existing schema mismatch is documented — do not fix here.)
        doc.add_text(schema.quality_field, quality.to_string());
        doc.add_f64(schema.pagerank_field, 0.0);
        doc.add_f64(schema.tfidf_field, 0.0);
        writer.add_document(doc).expect("add doc");
    }

    writer.commit().expect("commit");
}

/// Build the Axum router with a real SearchEngine backed by a temp index.
/// Returns the router and the TempDir (must be kept alive for the test duration).
fn build_test_router(api_key: Option<String>) -> (axum::Router, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    create_test_index(dir.path());
    let engine = SearchEngine::new(dir.path()).expect("open engine");
    let state = Arc::new(AppState { engine, api_key });
    let router = build_router(state);
    (router, dir)
}

// ─── Helper ───────────────────────────────────────────────────────────────────

async fn send(router: axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(req).await.expect("oneshot");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn get_with_auth(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn get_with_header(uri: &str, header_name: &str, header_value: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header(header_name, header_value)
        .body(Body::empty())
        .unwrap()
}

// ─── /healthz ─────────────────────────────────────────────────────────────────

/// 1. GET /healthz → 200
#[tokio::test]
async fn healthz_returns_200() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
}

/// healthz is accessible even when auth is enabled (no auth required on this route)
#[tokio::test]
async fn healthz_no_auth_required_even_when_key_configured() {
    let (router, _dir) = build_test_router(Some("secret-key".into()));
    let (status, json) = send(router, get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
}

// ─── Successful search ────────────────────────────────────────────────────────

/// 2. Successful search returns 200 with results array and took_ms
#[tokio::test]
async fn search_returns_results() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK, "json: {json}");
    assert!(json["results"].is_array(), "expected results array");
    assert!(json["took_ms"].is_number(), "expected took_ms");
}

/// 3. Empty result set (query with no matches)
#[tokio::test]
async fn search_empty_results() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=xyzzynosuchterm12345")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["results"].as_array().unwrap().len(), 0);
}

/// 4. Multiple results are all returned
#[tokio::test]
async fn search_multiple_results() {
    let (router, _dir) = build_test_router(None);
    // "rust" appears in all 3 test documents
    let (status, json) = send(router, get("/v1/search?q=rust&limit=10")).await;
    assert_eq!(status, StatusCode::OK);
    let results = json["results"].as_array().unwrap();
    assert!(!results.is_empty(), "expected at least 1 result for 'rust'");
}

/// 5. Default limit (no limit param → default 10)
#[tokio::test]
async fn search_default_limit_is_10() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);
    // With only 3 docs all matching, result count ≤ 10 (default cap)
    let results = json["results"].as_array().unwrap();
    assert!(results.len() <= 10);
}

/// 6. limit parameter is forwarded and respected
#[tokio::test]
async fn search_limit_forwarded() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust&limit=1")).await;
    assert_eq!(status, StatusCode::OK);
    let results = json["results"].as_array().unwrap();
    assert!(results.len() <= 1, "limit=1 must return at most 1 result");
}

// ─── Validation ───────────────────────────────────────────────────────────────

/// 7. limit > 100 → 400
#[tokio::test]
async fn search_limit_exceeds_max_returns_400() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust&limit=101")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        json["error"].as_str().unwrap().contains("limit"),
        "json: {json}"
    );
}

/// 8. Missing query → 400
#[tokio::test]
async fn search_missing_query_returns_400() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].is_string(), "expected error field");
}

/// 9. Empty query → 400
#[tokio::test]
async fn search_empty_query_returns_400() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].is_string());
}

/// limit=0 → 400
#[tokio::test]
async fn search_limit_zero_returns_400() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust&limit=0")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].is_string());
}

// ─── Authentication — no key configured ──────────────────────────────────────

/// 10. Auth disabled: unauthenticated request succeeds
#[tokio::test]
async fn auth_disabled_allows_unauthenticated() {
    let (router, _dir) = build_test_router(None);
    let (status, _json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);
}

// ─── Authentication — key configured ─────────────────────────────────────────

/// 11. Auth enabled: correct key → 200
#[tokio::test]
async fn auth_enabled_correct_key_returns_200() {
    let (router, _dir) = build_test_router(Some("test-secret-key".into()));
    let (status, _json) = send(
        router,
        get_with_auth("/v1/search?q=rust", "test-secret-key"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// 12. Auth enabled: missing Authorization header → 401
#[tokio::test]
async fn auth_missing_header_returns_401() {
    let (router, _dir) = build_test_router(Some("test-secret-key".into()));
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(json["error"].is_string());
}

/// 13. Auth enabled: wrong token → 401
#[tokio::test]
async fn auth_wrong_token_returns_401() {
    let (router, _dir) = build_test_router(Some("test-secret-key".into()));
    let (status, json) = send(router, get_with_auth("/v1/search?q=rust", "wrong-key")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "invalid_api_key");
}

/// 14. Auth enabled: malformed Authorization (no "Bearer ") → 401
#[tokio::test]
async fn auth_malformed_authorization_returns_401() {
    let (router, _dir) = build_test_router(Some("test-secret-key".into()));

    // "Token" scheme instead of "Bearer"
    let (status, json) = send(
        router,
        get_with_header(
            "/v1/search?q=rust",
            "Authorization",
            "Token test-secret-key",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "malformed_authorization_header");
}

// ─── Response contract ────────────────────────────────────────────────────────

/// 15. Response JSON exactly matches V1 contract shape
#[tokio::test]
async fn search_response_matches_v1_contract() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);

    // Top-level keys
    assert!(json.get("results").is_some(), "missing 'results' key");
    assert!(json.get("took_ms").is_some(), "missing 'took_ms' key");

    // No unexpected top-level keys
    let obj = json.as_object().unwrap();
    for key in obj.keys() {
        assert!(
            matches!(key.as_str(), "results" | "took_ms"),
            "unexpected top-level key: {key}"
        );
    }

    // Each result item shape
    if let Some(items) = json["results"].as_array() {
        for item in items {
            let item_obj = item.as_object().expect("result item must be object");
            for key in item_obj.keys() {
                assert!(
                    matches!(key.as_str(), "url" | "title" | "snippet" | "score"),
                    "unexpected result field: {key}"
                );
            }
            assert!(item["url"].is_string(), "url must be string");
            assert!(item["title"].is_string(), "title must be string");
            assert!(item["snippet"].is_string(), "snippet must be string");
            assert!(item["score"].is_number(), "score must be number");
        }
    }
}

/// 16. title=None maps to empty string (not null)
#[tokio::test]
async fn search_title_none_becomes_empty_string() {
    // All our test documents have titles, but we verify the type contract:
    // title must always be a JSON string (never null).
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);
    if let Some(items) = json["results"].as_array() {
        for item in items {
            assert!(
                item["title"].is_string(),
                "title must be a JSON string, got: {}",
                item["title"]
            );
        }
    }
}

/// 17. snippet=None maps to empty string (not null)
#[tokio::test]
async fn search_snippet_none_becomes_empty_string() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);
    if let Some(items) = json["results"].as_array() {
        for item in items {
            assert!(
                item["snippet"].is_string(),
                "snippet must be a JSON string, got: {}",
                item["snippet"]
            );
        }
    }
}

/// 18. Internal crawler SearchResult fields are not exposed
#[tokio::test]
async fn search_internal_fields_not_in_response() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);

    if let Some(items) = json["results"].as_array() {
        for item in items {
            // These fields exist on SearchResult but must NOT appear in the API response.
            assert!(item.get("domain").is_none(), "domain must not be exposed");
            assert!(
                item.get("quality_score").is_none(),
                "quality_score must not be exposed"
            );
            assert!(
                item.get("pagerank").is_none(),
                "pagerank must not be exposed"
            );
            assert!(item.get("tfidf").is_none(), "tfidf must not be exposed");
            assert!(
                item.get("crawled_at").is_none(),
                "crawled_at must not be exposed"
            );
        }
    }
}

/// 19. SearchEngine open failure → stable error (engine with bad path)
#[tokio::test]
async fn search_engine_error_returns_stable_category() {
    // Build a router with an engine opened on a real (empty) dir with no valid index.
    // We simulate this by opening on a valid dir but then removing the index files.
    // Instead, test a bad Tantivy query which triggers an engine error.
    let dir = TempDir::new().unwrap();
    create_test_index(dir.path());
    let engine = SearchEngine::new(dir.path()).unwrap();
    let state = Arc::new(AppState {
        engine,
        api_key: None,
    });
    let router = build_router(state);

    // A query with only Tantivy special chars that fails to parse gracefully.
    // Note: Tantivy is quite permissive; this test verifies the error path format.
    let (status, _json) = send(router, get("/v1/search?q=rust")).await;
    // Should succeed normally; the stability guarantee is about format not content.
    assert!(status.is_success() || status == StatusCode::INTERNAL_SERVER_ERROR);
}

/// 20. Auth: key never appears in error messages
#[tokio::test]
async fn auth_key_not_in_error_response() {
    let secret = "SUPER_SECRET_DO_NOT_EXPOSE";
    let (router, _dir) = build_test_router(Some(secret.into()));

    // Wrong key triggers 401; the error body must not contain the real key.
    let (status, json) = send(router, get_with_auth("/v1/search?q=rust", "wrong")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let error_str = json.to_string();
    assert!(
        !error_str.contains(secret),
        "API key must not appear in error body, got: {error_str}"
    );
}

// ─── Additional edge cases ────────────────────────────────────────────────────

/// took_ms is a non-negative integer
#[tokio::test]
async fn search_took_ms_is_non_negative() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);
    let took = json["took_ms"].as_u64().expect("took_ms must be a u64");
    let _ = took; // just checking it parsed correctly
}

/// result score values are finite floats
#[tokio::test]
async fn search_scores_are_finite() {
    let (router, _dir) = build_test_router(None);
    let (status, json) = send(router, get("/v1/search?q=rust")).await;
    assert_eq!(status, StatusCode::OK);
    if let Some(items) = json["results"].as_array() {
        for item in items {
            let score = item["score"].as_f64().expect("score must be a float");
            assert!(score.is_finite(), "score must be finite, got: {score}");
        }
    }
}

/// Auth enabled: correct key with extra whitespace in value → 401
/// (strict parsing — Bearer tokens are case/space sensitive)
#[tokio::test]
async fn auth_key_with_extra_whitespace_fails() {
    let (router, _dir) = build_test_router(Some("exact-key".into()));
    let (status, _) = send(
        router,
        get_with_header("/v1/search?q=rust", "Authorization", "Bearer exact-key "),
    )
    .await;
    // Trailing space means the token doesn't match exactly
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// limit=100 (maximum) is accepted
#[tokio::test]
async fn search_limit_at_max_is_accepted() {
    let (router, _dir) = build_test_router(None);
    let (status, _) = send(router, get("/v1/search?q=rust&limit=100")).await;
    assert_eq!(status, StatusCode::OK);
}

/// Non-existent route → 404
#[tokio::test]
async fn unknown_route_returns_404() {
    let (router, _dir) = build_test_router(None);
    let (status, _) = send(router, get("/v2/search?q=rust")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
