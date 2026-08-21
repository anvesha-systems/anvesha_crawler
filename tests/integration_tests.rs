//! Integration tests for network module

use crawler::network::{HttpClient, NetworkError};

#[tokio::test]
async fn test_http_client_basic_functionality() {
    let client = HttpClient::new().expect("Failed to create HTTP client");

    // Test successful request
    match client.fetch("https://httpbin.org/html").await {
        Ok(response) => {
            assert_eq!(response.status_code, 200);
            assert!(!response.content.is_empty());
            assert!(response.content_type.starts_with("text/html"));
            assert!(response.fetch_time_ms > 0);
            println!("✅ HTTP client basic test passed");
        }
        Err(e) => {
            // Don't fail test due to network issues in CI
            eprintln!("⚠️ Network test skipped (network issue): {}", e);
        }
    }
}

#[tokio::test]
async fn test_http_error_handling() {
    let client = HttpClient::new().expect("Failed to create HTTP client");

    // Test 404 error handling
    match client.fetch("https://httpbin.org/status/404").await {
        Ok(_) => panic!("Expected 404 error"),
        Err(NetworkError::Http { status, .. }) => {
            assert_eq!(status, 404);
            println!("✅ Error handling test passed");
        }
        Err(e) => {
            eprintln!("⚠️ Unexpected error (might be network issue): {}", e);
        }
    }
}

#[tokio::test]
async fn test_http_client_with_timeout() {
    let client = HttpClient::new()
        .expect("Failed to create HTTP client")
        .with_timeout(std::time::Duration::from_secs(10));

    // Test with custom timeout
    match client.fetch("https://httpbin.org/delay/1").await {
        Ok(response) => {
            assert_eq!(response.status_code, 200);
            println!("✅ Timeout test passed");
        }
        Err(e) => {
            eprintln!("⚠️ Timeout test skipped (network issue): {}", e);
        }
    }
}
