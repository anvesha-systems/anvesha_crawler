//! HTTP response processing and validation

use crate::network::NetworkError;
use encoding_rs::{Encoding, UTF_8};
use reqwest::{Response, header::HeaderMap};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub url: String,
    pub final_url: String,
    pub status_code: u16,
    pub headers: HeaderMap,
    pub content: String,
    pub content_type: String,
    pub content_length: Option<usize>,
    pub encoding: String,
    pub fetch_time_ms: u64,
    pub redirect_count: u32, // Fixed: f32 -> u32
}

pub struct ResponseProcessor {
    max_content_size: usize,
    allowed_content_types: Vec<String>,
}

impl ResponseProcessor {
    pub fn new() -> Self {
        Self {
            max_content_size: 10 * 1024 * 1024,
            allowed_content_types: vec![
                "text/html".to_string(),
                "application/xhtml+xml".to_string(),
                "text/plain".to_string(),
            ],
        }
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_content_size = size;
        self
    }

    pub fn with_allowed_content_types(mut self, types: Vec<String>) -> Self {
        self.allowed_content_types = types;
        self
    }

    /// Process reqwest Response into our HttpResponse
    pub async fn process_response(
        &self,
        response: Response,
        start_time: Instant,
        redirect_count: u32, // Fixed: f32 -> u32
    ) -> Result<HttpResponse, NetworkError> {
        let url = response.url().to_string();
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();

        // Validate status code
        if !response.status().is_success() {
            return Err(NetworkError::Http {
                status: status_code,
                message: format!("HTTP {} for {}", status_code, url),
            });
        }

        // Get content type
        let content_type = self.extract_content_type(&headers);

        // Validate content type
        if !self.is_allowed_content_type(&content_type) {
            return Err(NetworkError::UnsupportedContentType(content_type));
        }

        // Get response bytes
        let bytes = response
            .bytes()
            .await
            .map_err(|e| NetworkError::Request(e))?;

        // Check actual size
        if bytes.len() > self.max_content_size {
            return Err(NetworkError::ContentTooLarge {
                size: bytes.len(),
                limit: self.max_content_size,
            });
        }

        // Detect and convert encoding
        let (content, encoding) = self.decode_content(&bytes, &content_type)?; // Fixed: Added ?

        let fetch_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(HttpResponse {
            final_url: url.clone(),
            url,
            status_code,
            headers,
            content,
            content_type,
            content_length: Some(bytes.len()),
            encoding,
            fetch_time_ms,
            redirect_count,
        })
    }

    fn extract_content_type(&self, headers: &HeaderMap) -> String {
        headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_lowercase()) // Fixed: Added unwrap_or and trim
            .unwrap_or_else(|| "text/html".to_string())
    }

    fn extract_content_length(&self, headers: &HeaderMap) -> Option<usize> {
        // Fixed: Return type String -> Option<usize>
        headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    }

    fn is_allowed_content_type(&self, content_type: &str) -> bool {
        self.allowed_content_types
            .iter()
            .any(|allowed| content_type.starts_with(allowed))
    }

    fn decode_content(
        &self,
        bytes: &[u8],
        content_type: &str,
    ) -> Result<(String, String), NetworkError> {
        // Try to detect encoding from content type
        let encoding = self.detect_encoding(bytes, content_type);

        let (decoded, _encoding_used, had_errors) = encoding.decode(bytes); // Fixed: decode -> decoded

        if had_errors {
            tracing::warn!("Encoding errors detected while decoding content");
        }

        Ok((decoded.to_string(), encoding.name().to_string()))
    }

    fn detect_encoding(&self, bytes: &[u8], content_type: &str) -> &'static Encoding {
        // Try to extract charset from Content-Type header
        if let Some(charset) = self.extract_charset_content_type(content_type) {
            if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
                // Fixed: from_label -> for_label
                return encoding;
            }
        }

        // Try to detect from HTML meta tags
        if let Some(encoding) = self.detect_html_encoding(bytes) {
            return encoding;
        }

        // Try to detect from BOM
        if let Some(encoding) = self.detect_bom_encoding(bytes) {
            return encoding;
        }

        // Default to UTF-8
        UTF_8
    }

    fn extract_charset_content_type(&self, content_type: &str) -> Option<String> {
        content_type.split(';').find_map(|part| {
            let trimmed = part.trim();
            if trimmed.starts_with("charset=") {
                Some(trimmed[8..].trim().to_string()) // Fixed: Added trim()
            } else {
                None
            }
        })
    }

    fn detect_html_encoding(&self, bytes: &[u8]) -> Option<&'static Encoding> {
        // Look for <meta charset="..."> or <meta http-equiv="Content-Type" content="...">
        let text = std::str::from_utf8(&bytes[..std::cmp::min(bytes.len(), 1024)]).ok()?;

        // Simple regex-like search for charset
        if let Some(start) = text.find("charset=") {
            let charset_part = &text[start + 8..];
            let end = charset_part
                .find('"')
                .or_else(|| charset_part.find('\''))
                .or_else(|| charset_part.find(' '))
                .or_else(|| charset_part.find('>'))
                .unwrap_or(charset_part.len());

            let charset = &charset_part[..end].trim();
            return Encoding::for_label(charset.as_bytes()); // Fixed: from_label -> for_label
        }
        None
    }

    fn detect_bom_encoding(&self, bytes: &[u8]) -> Option<&'static Encoding> {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            Some(UTF_8)
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            Some(encoding_rs::UTF_16LE)
        } else if bytes.starts_with(&[0xFE, 0xFF]) {
            Some(encoding_rs::UTF_16BE)
        } else {
            None
        }
    }
}

impl Default for ResponseProcessor {
    fn default() -> Self {
        Self::new()
    }
}
