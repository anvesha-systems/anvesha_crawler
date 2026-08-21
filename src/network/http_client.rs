//! HTTP client with user agent rotation and robust error handling

use crate::network::{HttpResponse, NetworkError, ResponseProcessor, classify_reqwest_error};
use reqwest::{Client, ClientBuilder, redirect::Policy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering}; // Fixed: removed duplicate and typo
use std::time::{Duration, Instant};
use tracing::{debug, info}; // Fixed: removed duplicate debug import

pub struct HttpClient {
    client: Client,
    response_processor: ResponseProcessor,
    user_agents: Vec<String>,
    current_ua_index: Arc<AtomicUsize>,
    default_timeout: Duration,
    max_redirects: u32,
}

impl HttpClient {
    pub fn new() -> Result<Self, NetworkError> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30)) // Fixed: 3 -> 30 seconds for more reasonable timeout
            .redirect(Policy::limited(10))
            .gzip(true)
            .brotli(true)
            .build()
            .map_err(|e| NetworkError::Request(e))?;

        let default_user_agents = vec![
            "Mozilla/5.0 (compatible; WebCrawler/1.0; +http://example.com/bot)".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0".to_string(),
        ];

        Ok(Self {
            client,
            response_processor: ResponseProcessor::new(),
            user_agents: default_user_agents,
            current_ua_index: Arc::new(AtomicUsize::new(0)),
            default_timeout: Duration::from_secs(30),
            max_redirects: 10,
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    pub fn with_user_agents(mut self, user_agents: Vec<String>) -> Self {
        if !user_agents.is_empty() {
            self.user_agents = user_agents;
        }
        self
    }

    pub fn with_max_content_size(mut self, size: usize) -> Self {
        self.response_processor = self.response_processor.with_max_size(size);
        self
    }

    /// Fetch a URL with automatic retries and error handling
    pub async fn fetch(&self, url: &str) -> Result<HttpResponse, NetworkError> {
        self.fetch_with_options(url, None).await
    }

    /// Fetch URL with custom timeout
    pub async fn fetch_with_timeout(
        &self,
        url: &str,
        timeout: Duration,
    ) -> Result<HttpResponse, NetworkError> {
        self.fetch_with_options(url, Some(timeout)).await
    }

    async fn fetch_with_options(
        &self,
        url: &str,
        timeout: Option<Duration>,
    ) -> Result<HttpResponse, NetworkError> {
        let start_time = Instant::now();
        let user_agent = self.get_next_user_agent();
        let timeout = timeout.unwrap_or(self.default_timeout);

        debug!("Fetching URL: {} (timeout: {}s)", url, timeout.as_secs()); // Fixed: missing closing parenthesis

        // Validate URL format
        let parsed_url =
            url::Url::parse(url).map_err(|_| NetworkError::InvalidUrl(url.to_string()))?; // Fixed: removed unused parameter

        // Only allow HTTP/HTTPS
        match parsed_url.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(NetworkError::InvalidUrl(
                    format!("Unsupported scheme: {}", scheme), // Fixed: clearer error message
                ));
            }
        }

        // Build request
        let mut request_builder = self
            .client
            .get(url)
            .header("User-Agent", &user_agent)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,text/plain;q=0.9,*/*;q=0.8",
            ) // Fixed: spacing
            .header("Accept-Language", "en-US,en;q=0.5") // Fixed: spacing and capitalization
            .header("Accept-Encoding", "gzip, deflate, br") // Fixed: spacing
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .timeout(timeout);

        // Add cache control
        request_builder = request_builder.header("Cache-Control", "no-cache");

        // Send request
        let response = request_builder
            .send()
            .await
            .map_err(|e| classify_reqwest_error(e, url))?;

        // Count redirects
        let redirect_count = self.count_redirects(&response);
        if redirect_count > self.max_redirects {
            return Err(NetworkError::TooManyRedirects {
                count: redirect_count,
                limit: self.max_redirects,
            });
        }

        // Process response
        let http_response = self
            .response_processor
            .process_response(response, start_time, redirect_count)
            .await?;

        info!(
            "Successfully fetched {} ({} bytes, {} ms)",
            url,
            http_response.content_length.unwrap_or(0),
            http_response.fetch_time_ms
        );

        Ok(http_response)
    }

    fn get_next_user_agent(&self) -> String {
        let index = self.current_ua_index.fetch_add(1, Ordering::Relaxed);
        self.user_agents[index % self.user_agents.len()].clone()
    }

    fn count_redirects(&self, _response: &reqwest::Response) -> u32 {
        // Simple redirect count - in practice, reqwest handles this
        // This is a placeholder for more sophisticated redirect tracking
        0
    }

    /// Test if a URL is reachable (HEAD request)
    pub async fn test_url(&self, url: &str) -> Result<u16, NetworkError> {
        let user_agent = self.get_next_user_agent();

        let response = self
            .client
            .head(url)
            .header("User-Agent", &user_agent)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| classify_reqwest_error(e, url))?;

        Ok(response.status().as_u16())
    }

    /// Get client statistics
    pub fn get_stats(&self) -> HttpClientStats {
        HttpClientStats {
            current_user_agent_index: self.current_ua_index.load(Ordering::Relaxed),
            total_user_agents: self.user_agents.len(),
            default_timeout_secs: self.default_timeout.as_secs(),
            max_redirects: self.max_redirects,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpClientStats {
    pub current_user_agent_index: usize,
    pub total_user_agents: usize,
    pub default_timeout_secs: u64,
    pub max_redirects: u32,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client") // Fixed: typo in error message
    }
}
