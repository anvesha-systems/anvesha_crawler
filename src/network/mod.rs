//! Network module for HTTP requests and response handling

pub mod error_handler;
pub mod http_client;
pub mod response_handler;

// Re-export the main types
pub use error_handler::{NetworkError, classify_reqwest_error};
pub use http_client::{HttpClient, HttpClientStats};
pub use response_handler::{HttpResponse, ResponseProcessor};

// Tests module
#[cfg(test)]
mod tests;
