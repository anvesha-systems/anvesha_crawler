//! Manages the URL queue with prioritization

use crate::models::CrawlUrl;
use dashmap::DashSet;
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::Mutex; // Changed: std::sync::Mutex -> tokio::sync::Mutex (for async)
use tracing::debug;

/// Thread-safe URL frontier that manages crawling queue with prioritization
pub struct UrlFrontier {
    /// Priority queue for URLs to crawl (higher priority first)
    queue: Arc<Mutex<BinaryHeap<CrawlUrl>>>,

    /// Set of URLs already seen to avoid duplicates
    seen_urls: Arc<DashSet<String>>,

    /// Set of URLs already crawled
    crawled_urls: Arc<DashSet<String>>, // Fixed: crwaled_urls -> crawled_urls

    /// Maximum queue size to prevent memory issues
    max_queue_size: usize,
}

impl UrlFrontier {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            seen_urls: Arc::new(DashSet::new()),
            crawled_urls: Arc::new(DashSet::new()), // Fixed: crwaled_urls -> crawled_urls
            max_queue_size,
        }
    }

    /// Add URL to frontier if not already seen
    pub async fn add_url(&self, url: CrawlUrl) -> bool {
        if self.seen_urls.contains(&url.url) {
            return false;
        }

        let mut queue = self.queue.lock().await;

        // Check queue size limit
        if queue.len() >= self.max_queue_size {
            debug!("URL frontier limit exceeded, dropping URL: {}", url.url);
            return false;
        }

        self.seen_urls.insert(url.url.clone());
        queue.push(url);
        true
    }

    /// Add multiple URLs at once
    pub async fn add_urls(&self, urls: Vec<CrawlUrl>) -> usize {
        let mut added = 0;
        for url in urls {
            if self.add_url(url).await {
                added += 1;
            }
        }
        added
    }

    /// Get next URL to crawl (highest priority)
    pub async fn next_url(&self) -> Option<CrawlUrl> {
        let mut queue = self.queue.lock().await;
        queue.pop()
    }

    /// Mark URL as crawled
    pub fn mark_crawled(&self, url: &str) {
        self.crawled_urls.insert(url.to_string()); // Fixed: crwaled_urls -> crawled_urls
    }

    /// Check if URL is already crawled
    pub fn is_crawled(&self, url: &str) -> bool {
        self.crawled_urls.contains(url) // Fixed: crwaled_urls -> crawled_urls
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> FrontierStats {
        let queue_size = self.queue.lock().await.len();
        FrontierStats {
            queue_size,
            seen_count: self.seen_urls.len(),
            crawled_count: self.crawled_urls.len(), // Fixed: crwaled_urls -> crawled_urls
        }
    }

    /// Check if frontier is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct FrontierStats {
    pub queue_size: usize,    // Changed: pub(crate) -> pub
    pub seen_count: usize,    // Changed: pub(crate) -> pub
    pub crawled_count: usize, // Added: pub visibility
}
