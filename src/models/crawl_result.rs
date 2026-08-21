use crate::models::page_data::PageData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlResult {
    Success(PageData),
    Failed {
        url: String,
        error: String,
        retry_count: u32,
    },

    Skipped {
        url: String,
        reason: String,
    },
}
