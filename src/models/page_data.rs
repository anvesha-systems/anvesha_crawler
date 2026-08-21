use crate::models::crawl_url::CrawlUrl;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageData {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub content: String,
    pub outgoing_links: Vec<CrawlUrl>,
    pub word_count: usize,
    pub content_quality_score: f64,
    pub crawled_at: chrono::DateTime<chrono::Utc>,
    pub depth: u32,
}
