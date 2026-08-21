// in memory caching implementaion

use crate::storage::models::StoredPage;
use crate::storage::{Result, StorageError};
use moka::sync::Cache as MokaCache;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

// In memory cache implementaion using Moka
pub struct MemoryCache {
    // Page cache
    page_cache: MokaCache<String, StoredPage>,

    // Search result cache
    search_cache: MokaCache<String, Vec<StoredPage>>,

    // URL existance cache (for duplocate detection)
    url_cache: MokaCache<String, bool>,

    // General purpose cache for serializable data
    general_cache: MokaCache<String, String>, // json string

    // Configuration
    default_ttl: Duration,
}

impl MemoryCache {
    // Create a new memory cache
    pub fn new(max_capaciity: u64, default_ttl: Duration) -> Self {
        info!(
            "Initializing memory cache with capacity : {}, TTL: {:?}",
            max_capaciity, default_ttl
        );

        Self {
            page_cache: MokaCache::builder()
                .max_capacity(max_capaciity / 4)
                .time_to_live(default_ttl)
                .build(),

            search_cache: MokaCache::builder()
                .max_capacity(max_capaciity / 4)
                .time_to_live(Duration::from_secs(300))
                .build(),

            url_cache: MokaCache::builder()
                .max_capacity(max_capaciity / 2)
                .time_to_live(default_ttl)
                .build(),

            general_cache: MokaCache::builder()
                .max_capacity(max_capaciity / 4)
                .time_to_live(default_ttl)
                .build(),

            default_ttl,
        }
    }

    // create a cache with default settings
    pub fn default() -> Self {
        Self::new(10_000, Duration::from_secs(3600)) // 10k entries, 1 hr ttl
    }

    // page cache methods

    // cache a page
    pub fn cache_page(&self, page: &StoredPage) {
        let key = format!("Page: {}", page.id);
        self.page_cache.insert(key.clone(), page.clone());

        // Also cache URL -> ID mapping
        let url_key = format!("URL: {}", page.url);
        self.page_cache.insert(url_key, page.clone());

        debug!("Cached page: {} (ID : {})", page.url, page.id);
    }

    // Get a page by ID from cache
    pub fn get_page_by_id(&self, page_id: i64) -> Option<StoredPage> {
        let key = format!("Page: {}", page_id);
        let result = self.page_cache.get(&key);

        if result.is_some() {
            debug!("Cache hit for page ID: {}", page_id);
        }
        result
    }

    // get a page by URL from cache
    pub fn get_page_by_url(&self, url: &str) -> Option<StoredPage> {
        let key = format!("URL: {}", url);
        let result = self.page_cache.get(&key);

        if result.is_some() {
            debug!("Cache hit for page URL: {}", url);
        }

        result
    }

    // URL Existance caching (for duplicate detection)

    // Cache url existance
    pub fn cache_url_exists(&self, url: &str, exists: bool) {
        self.url_cache.insert(url.to_string(), exists);
        debug!("Cached URL existance {}: {}", url, exists);
    }

    // check if URL existance is  cached
    pub fn get_url_exists(&self, url: &str) -> Option<bool> {
        self.url_cache.get(url)
    }

    // search result caching

    // caching search results
    pub fn cache_search_results(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        results: &[StoredPage],
    ) {
        let key = format!("Search : {} : {} : {}", query, limit, offset);
        self.search_cache.insert(key, results.to_vec());
        debug!(
            "Cache search results for query : {} ({} results)",
            query,
            results.len()
        );
    }

    // Get cached search results
    pub fn get_search_results(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Option<Vec<StoredPage>> {
        let key = format!("Search : {} : {} : {}", query, limit, offset);
        let result = self.search_cache.get(&key);

        if result.is_some() {
            debug!("Cache hit for search: {}", query);
        }

        result
    }

    // General purpose caching

    // set a value in general cahce
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json_value =
            serde_json::to_string(value).map_err(|e| StorageError::Serialization(e))?;

        self.general_cache.insert(key.to_string(), json_value);
        debug!("Cached value for key: {}", key);
        Ok(())
    }

    /// Get a value from the general cache
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        if let Some(json_value) = self.general_cache.get(key) {
            let value =
                serde_json::from_str(&json_value).map_err(|e| StorageError::Serialization(e))?;
            debug!("Cache hit for key: {}", key);
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Remove a value from all caches
    pub fn invalidate(&self, key: &str) {
        // Try to remove from all caches
        self.general_cache.invalidate(key);
        self.page_cache.invalidate(key);
        self.search_cache.invalidate(key);
        self.url_cache.invalidate(key);
        debug!("Invalidated cache key: {}", key);
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.page_cache.invalidate_all();
        self.search_cache.invalidate_all();
        self.url_cache.invalidate_all();
        self.general_cache.invalidate_all();
        info!("Cleared all caches");
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        // Flush pending Moka housekeeping so entry_count() is accurate.
        self.run_pending_tasks();
        CacheStats {
            page_cache_size: self.page_cache.entry_count(),
            search_cache_size: self.search_cache.entry_count(),
            url_cache_size: self.url_cache.entry_count(),
            general_cache_size: self.general_cache.entry_count(),
            total_entries: self.page_cache.entry_count()
                + self.search_cache.entry_count()
                + self.url_cache.entry_count()
                + self.general_cache.entry_count(),
        }
    }

    /// Run cache maintenance (cleanup expired entries)
    pub fn run_pending_tasks(&self) {
        self.page_cache.run_pending_tasks();
        self.search_cache.run_pending_tasks();
        self.url_cache.run_pending_tasks();
        self.general_cache.run_pending_tasks();
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub page_cache_size: u64,
    pub search_cache_size: u64,
    pub url_cache_size: u64,
    pub general_cache_size: u64,
    pub total_entries: u64,
}

/// Cached search query key
fn search_key(query: &str, limit: usize, offset: usize) -> String {
    format!("search:{}:{}:{}", query, limit, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StoredPage;
    use chrono::Utc;

    #[test]
    fn test_memory_cache_creation() {
        let cache = MemoryCache::new(1000, Duration::from_secs(60));
        let stats = cache.get_stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_page_caching() {
        let cache = MemoryCache::default();

        let page = StoredPage {
            id: 1,
            url: "https://example.com".to_string(),
            url_hash: "hash123".to_string(),
            domain: "example.com".to_string(),
            title: Some("Test Page".to_string()),
            description: None,
            content: "Test content".to_string(),
            content_hash: "content_hash".to_string(),
            quality_score: 0.8,
            word_count: 2,
            language: "en".to_string(),
            crawl_depth: 1,
            crawled_at: Utc::now(),
            last_modified: None,
            status_code: 200,
            content_type: "text/html".to_string(),
            content_length: 12,
            pagerank: None,
            tfidf_score: None,
        };

        // Cache the page
        cache.cache_page(&page);

        // Test retrieval by ID
        let cached_by_id = cache.get_page_by_id(1);
        assert!(cached_by_id.is_some());
        assert_eq!(cached_by_id.unwrap().url, page.url);

        // Test retrieval by URL
        let cached_by_url = cache.get_page_by_url("https://example.com");
        assert!(cached_by_url.is_some());
        assert_eq!(cached_by_url.unwrap().id, 1);

        // Test cache stats
        let stats = cache.get_stats();
        assert!(stats.page_cache_size >= 2); // At least 2 entries (ID and URL keys)
    }

    #[test]
    fn test_url_existence_caching() {
        let cache = MemoryCache::default();

        // Cache URL existence
        cache.cache_url_exists("https://example.com", true);
        cache.cache_url_exists("https://nonexistent.com", false);

        // Test retrieval
        assert_eq!(cache.get_url_exists("https://example.com"), Some(true));
        assert_eq!(cache.get_url_exists("https://nonexistent.com"), Some(false));
        assert_eq!(cache.get_url_exists("https://unknown.com"), None);
    }

    #[test]
    fn test_general_caching() {
        let cache = MemoryCache::default();

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            value: i32,
        }

        let test_data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        // Cache the data
        cache.set("test_key", &test_data).unwrap();

        // Retrieve the data
        let cached_data: Option<TestData> = cache.get("test_key").unwrap();
        assert!(cached_data.is_some());
        assert_eq!(cached_data.unwrap(), test_data);

        // Test non-existent key
        let non_existent: Option<TestData> = cache.get("non_existent").unwrap();
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = MemoryCache::default();

        // Set some data
        cache.set("test", &"value".to_string()).unwrap();
        assert!(cache.get::<String>("test").unwrap().is_some());

        // Invalidate
        cache.invalidate("test");
        assert!(cache.get::<String>("test").unwrap().is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = MemoryCache::default();

        // Add some data
        cache.set("key1", &"value1".to_string()).unwrap();
        cache.set("key2", &"value2".to_string()).unwrap();

        let stats_before = cache.get_stats();
        assert!(stats_before.total_entries > 0);

        // Clear all
        cache.clear_all();

        let stats_after = cache.get_stats();
        assert_eq!(stats_after.total_entries, 0);
    }
}
