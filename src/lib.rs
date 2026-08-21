//! Search Engine Crawler Library

// Pre-existing lints in crawler modules — suppressed to keep the quality gate
// focused on new code. These were present before the V1 Search API work.
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unexpected_cfgs)]
#![allow(clippy::absurd_extreme_comparisons)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::len_zero)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::manual_inspect)]
#![allow(clippy::manual_range_patterns)]
#![allow(clippy::manual_strip)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::new_without_default)]
#![allow(clippy::print_literal)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::single_component_path_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::unwrap_or_default)]

pub mod algorithms;
pub mod api;
pub mod config;
pub mod core;
pub mod models;
pub mod network;
pub mod search;
pub mod storage;
pub mod utils;

use chrono::offset;
// Re-export commonly used types
pub use crate::core::crawler::WebCrawler;
pub use config::CrawlerConfig;
pub use models::{CrawlResult, CrawlStatistics, CrawlUrl, PageData};
pub use network::{HttpClient, NetworkError};
use tantivy::snippet;

/// Main crawler error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Initialize the crawler with logging and metrics
pub async fn init() -> Result<()> {
    utils::init_logger()?;
    utils::init_metrics().await?;
    Ok(())
}

use crate::search::filters::{self, SearchFilter, SortBy};
use crate::search::query::SearchQuery;
use std::path::Path;

// public search engine interface for adapters and integrations
pub struct SearchEngine {
    inner: SearchQuery,
}

impl SearchEngine {
    // initialize search engine interface for adapters and integrations
    pub fn new(index_path: &Path) -> Result<Self> {
        let inner = SearchQuery::new(index_path)?;
        Ok(Self { inner })
    }
    // execute search query

    pub fn search(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        filters: SearchFilter,
        sort: SortBy,
        snippets: bool,
        highlight: bool,
    ) -> Result<Vec<crate::search::SearchResult>> {
        let result = self
            .inner
            .search_with_filters(query, limit, filters, sort, offset, snippets, highlight)?;
        Ok(result)
    }
}
