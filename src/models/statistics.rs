use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlStatistics {
    pub pages_crawled: usize,
    pub pages_failed: usize,
    pub urls_discovered: usize,
    pub urls_in_queue: usize,
    pub elapsed_time: Duration,
    pub crawl_rate: f64,
}
