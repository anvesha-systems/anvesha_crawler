//! Network error handling and classification

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("HTTP error {status}: {message}")]
    Http { status: u16, message: String }, // Fixed: status should be u16, not String

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Content encoding error: {0}")]
    Encoding(String),

    #[error("Content too large: {size} bytes (limit: {limit})")]
    ContentTooLarge { size: usize, limit: usize },

    #[error("Unsupported content type: {0}")]
    UnsupportedContentType(String),

    #[error("Robots.txt disallows crawling: {0}")]
    RobotsDisallowed(String),

    #[error("Rate limit exceeded for domain: {0}")]
    RateLimited(String),

    #[error("DNS resolution failed: {0}")]
    DnsError(String),

    #[error("SSL/TLS error: {0}")]
    TlsError(String),

    #[error("Redirect loop detected: {0}")]
    RedirectLoop(String),

    #[error("Too many redirects: {count} (limit: {limit})")]
    TooManyRedirects { count: u32, limit: u32 },

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl NetworkError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            NetworkError::Timeout(_) => true,
            NetworkError::Connection(_) => true,
            NetworkError::DnsError(_) => false, // DNS errors rarely resolve quickly
            NetworkError::Http { status, .. } => {
                // Retry on server errors, not client errors
                *status >= 500 && *status < 600 || *status == 429 // Fixed: now status is u16, this works
            }
            NetworkError::RateLimited(_) => true,
            NetworkError::TlsError(_) => false,
            NetworkError::Request(e) => {
                // Check reqwest error type
                e.is_timeout() || e.is_connect()
            }
            _ => false,
        }
    }

    /// Get suggested retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> u64 {
        match self {
            NetworkError::Timeout(_) => 2000,
            NetworkError::Connection(_) => 1000,
            NetworkError::Http { status, .. } => {
                match *status {
                    // Fixed: now status is u16, this works
                    429 => 5000,             // Rate limited - wait longer
                    502 | 503 | 504 => 3000, // Server issues
                    _ => 1000,
                }
            }
            NetworkError::RateLimited(_) => 5000,
            _ => 1000, // Fixed: changed default from 5000 to 1000 for consistency
        }
    }
}

/// Convert reqwest errors to NetworkError with context
pub fn classify_reqwest_error(error: reqwest::Error, url: &str) -> NetworkError {
    if error.is_timeout() {
        NetworkError::Timeout(url.to_string())
    } else if error.is_connect() {
        NetworkError::Connection(format!("Failed to connect to {}", url))
    } else if let Some(status) = error.status() {
        NetworkError::Http {
            status: status.as_u16(), // Fixed: This now matches the u16 type
            message: format!("{}: {}", status, url),
        }
    } else if error.is_request() {
        NetworkError::InvalidUrl(url.to_string())
    } else {
        NetworkError::Request(error)
    }
}
