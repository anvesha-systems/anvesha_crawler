// database models and strucutures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Stored page in database

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StoredPage {
    pub id: i64,
    pub url: String,
    pub url_hash: String,
    pub domain: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: String,
    pub content_hash: String,
    pub quality_score: f64,
    pub word_count: i32,
    pub language: String,
    pub crawl_depth: i32,
    pub crawled_at: DateTime<Utc>,
    pub last_modified: Option<DateTime<Utc>>,
    pub status_code: i32,
    pub content_type: String,
    pub content_length: i32,

    #[sqlx(rename = "pagerank")]
    pub pagerank: Option<f64>,

    #[sqlx(rename = "tfidf_score")]
    pub tfidf_score: Option<f64>,
}

impl StoredPage {
    // Create a new Stored page from page data
    pub fn from_page_data(
        page: &crate::models::PageData,
        url_hash: String,
        content_hash: String,
    ) -> Self {
        let domain = page.url.split('/').nth(2).unwrap_or("unknown").to_string();

        Self {
            id: 0, // will be set by db
            url: page.url.clone(),
            url_hash,
            domain,
            title: page.title.clone(),
            description: page.description.clone(),
            content: page.content.clone(),
            content_hash,
            quality_score: page.content_quality_score,
            word_count: page.word_count as i32,
            language: "en".to_string(), //TODO: detect language
            crawl_depth: page.depth as i32,
            crawled_at: page.crawled_at,
            last_modified: None,
            status_code: 200, //TODO: get this from HTTP response
            content_type: "text/html".to_string(),
            content_length: page.content.len() as i32,
            pagerank: None,
            tfidf_score: None,
        }
    }

    // Convert page data for compatibality
    pub fn to_page_data(&self) -> crate::models::PageData {
        crate::models::PageData {
            url: self.url.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            keywords: vec![], // TODO: extract from stored data
            content: self.content.clone(),
            outgoing_links: vec![], //Would need to query liked table
            word_count: self.word_count as usize,
            content_quality_score: self.quality_score,
            crawled_at: self.crawled_at,
            depth: self.crawl_depth as u32,
        }
    }
}

// Link between pages
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StoredLink {
    pub id: i64,
    pub source_page_id: i64,
    pub target_url: String,
    pub target_page_id: Option<i64>,
    pub anchor_text: Option<String>,
    pub link_position: i32,
    pub crawled_at: DateTime<Utc>,
}

// crawl session tracking
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrawlSession {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub pages_crawled: i32,
    pub pages_failed: i32,
    pub seed_urls: String,       //JSON encoded
    pub config_snapshot: String, //JSON encoded
    pub status: String,
}

impl CrawlSession {
    // crate a new crawl session
    pub fn new(
        seed_urls: &[String],
        config: &crate::config::CrawlerConfig,
    ) -> crate::storage::Result<Self> {
        Ok(Self {
            id: 0, //will be set by db
            started_at: Utc::now(),
            ended_at: None,
            pages_crawled: 0,
            pages_failed: 0,
            seed_urls: serde_json::to_string(seed_urls)?,
            config_snapshot: serde_json::to_string(config)?,
            status: "running".to_string(),
        })
    }

    // Get seed URL's as vector
    pub fn get_seed_urls(&self) -> crate::storage::Result<Vec<String>> {
        Ok(serde_json::from_str(&self.seed_urls)?)
    }

    // Mark session as completed
    pub fn mark_completed(&mut self) {
        self.ended_at = Some(Utc::now());
        self.status = "completed".to_string();
    }

    // mark session as failed
    pub fn mark_failed(&mut self) {
        self.ended_at = Some(Utc::now());
        self.status = "failed".to_string();
    }
}

// Domain information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DomainInfo {
    pub domain: String,
    pub robots_txt: Option<String>,
    pub robots_fetched_at: Option<DateTime<Utc>>,
    pub crawl_delay: i32,
    pub page_count: i32,
    pub avg_quality_score: Option<f64>,
    pub last_crawled: Option<DateTime<Utc>>,
    pub crawl_allowed: bool,
}

impl DomainInfo {
    // create a new domain info
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            robots_txt: None,
            robots_fetched_at: None,
            crawl_delay: 1000, // 1 second default
            page_count: 0,
            avg_quality_score: None,
            last_crawled: None,
            crawl_allowed: true,
        }
    }
}

// search result with relevence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub page: StoredPage,
    pub score: f32,
    pub snippet: String,
    pub highlighted_fields: Vec<String>,
}

impl SearchResult {
    // Create new search result
    pub fn new(page: StoredPage, score: f32, snippet: String) -> Self {
        Self {
            page,
            score,
            snippet,
            highlighted_fields: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_pages: i64,
    pub total_links: i64,
    pub total_domains: i64,
    pub avg_quality_score: Option<f64>,
    pub crawl_sessions: i64,
    pub database_size_mb: f64,
}

impl Default for DatabaseStats {
    fn default() -> Self {
        Self {
            total_pages: 0,
            total_links: 0,
            total_domains: 0,
            avg_quality_score: None,
            crawl_sessions: 0,
            database_size_mb: 0.0,
        }
    }
}

// Page quality filter
#[derive(Debug, Clone)]
pub struct PageFilter {
    pub domain: Option<String>,
    pub min_quality: Option<f64>,
    pub max_quality: Option<f64>,
    pub crawled_after: Option<DateTime<Utc>>,
    pub crawled_before: Option<DateTime<Utc>>,
    pub status_code: Option<i32>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl Default for PageFilter {
    fn default() -> Self {
        Self {
            domain: None,
            min_quality: None,
            max_quality: None,
            crawled_after: None,
            crawled_before: None,
            status_code: None,
            limit: None,
            offset: None,
        }
    }
}

impl PageFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }

    pub fn with_min_quality(mut self, quality: f64) -> Self {
        self.min_quality = Some(quality);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PageData;

    #[test]
    fn test_stored_page_from_page_data() {
        let page_data = PageData {
            url: "https://example.com/test".to_string(),
            title: Some("Test Page".to_string()),
            description: Some("A test page".to_string()),
            keywords: vec!["test".to_string()],
            content: "Test content".to_string(),
            outgoing_links: vec![],
            word_count: 2,
            content_quality_score: 0.8,
            crawled_at: Utc::now(),
            depth: 1,
        };

        let stored_page = StoredPage::from_page_data(
            &page_data,
            "hash123".to_string(),
            "content_hash".to_string(),
        );

        assert_eq!(stored_page.url, page_data.url);
        assert_eq!(stored_page.title, page_data.title);
        assert_eq!(stored_page.quality_score, page_data.content_quality_score);
        assert_eq!(stored_page.domain, "example.com");
    }

    #[test]
    fn test_crawl_session_creation() {
        let seed_urls = vec!["https://example.com".to_string()];
        let config = crate::config::CrawlerConfig::default();

        let session = CrawlSession::new(&seed_urls, &config).unwrap();

        assert_eq!(session.status, "running");
        assert!(session.ended_at.is_none());
        assert!(session.ended_at.is_none());

        let parsed_urls = session.get_seed_urls().unwrap();
        assert_eq!(parsed_urls, seed_urls);
    }

    #[test]
    fn test_page_filter_builder() {
        let filter = PageFilter::new()
            .with_domain("example.com".to_string())
            .with_min_quality(0.5)
            .with_limit(100);

        assert_eq!(filter.domain, Some("example.com".to_string()));
        assert_eq!(filter.min_quality, Some(0.5));
        assert_eq!(filter.limit, Some(100));
    }
}
