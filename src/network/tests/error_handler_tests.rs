use crate::network::{NetworkError, classify_reqwest_error};

#[test]
fn test_network_error_retry_logic() {
    // Test retryable errors
    let timeout_error = NetworkError::Timeout("https://example.com".to_string());
    assert!(timeout_error.is_retryable());
    assert_eq!(timeout_error.retry_delay_ms(), 2000);

    let connection_error = NetworkError::Connection("Failed to connect".to_string());
    assert!(connection_error.is_retryable());
    assert_eq!(connection_error.retry_delay_ms(), 1000);

    // Test non-retryable errors
    let dns_error = NetworkError::DnsError("DNS failed".to_string());
    assert!(!dns_error.is_retryable());

    let invalid_url = NetworkError::InvalidUrl("bad-url".to_string());
    assert!(!invalid_url.is_retryable());
}

#[test]
fn test_http_error_classification() {
    // Test server errors (retryable)
    let server_error = NetworkError::Http {
        status: 500,
        message: "Internal Server Error".to_string(),
    };
    assert!(server_error.is_retryable());
    assert_eq!(server_error.retry_delay_ms(), 1000);

    // Test client errors (not retryable)
    let client_error = NetworkError::Http {
        status: 404,
        message: "Not Found".to_string(),
    };
    assert!(!client_error.is_retryable());

    // Test rate limiting
    let rate_limit_error = NetworkError::Http {
        status: 429,
        message: "Too Many Requests".to_string(),
    };
    assert!(rate_limit_error.is_retryable());
    assert_eq!(rate_limit_error.retry_delay_ms(), 5000);
}
