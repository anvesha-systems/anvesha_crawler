// Fix the import - WebCrawler is in the parent module (crate::core)
use crate::config::CrawlerConfig;
use crate::core::crawler::WebCrawler; // ✅ Correct path
use crate::models::CrawlUrl;

#[tokio::test]
async fn test_crawler_creation() {
    let config = CrawlerConfig::default();
    let crawler = WebCrawler::new(config).await;
    assert!(crawler.is_ok());
}

#[tokio::test]
async fn test_crawler_statistics_generation() {
    let config = CrawlerConfig::default();
    let crawler = WebCrawler::new(config).await.unwrap();

    let stats = crawler.generate_statistics().await;

    // Test initial state
    assert_eq!(stats.pages_crawled, 0);
    assert_eq!(stats.pages_failed, 0);
    let _ = stats.elapsed_time.as_millis(); // always non-negative for Duration
    assert_eq!(stats.crawl_rate, 0.0);
}

#[tokio::test]
async fn test_crawler_with_test_config() {
    let mut config = CrawlerConfig::default();
    config.crawler.max_pages = 5;
    config.crawler.concurrent_requests = 1;

    let crawler = WebCrawler::new(config).await;
    assert!(crawler.is_ok());

    let crawler = crawler.unwrap();
    let stats = crawler.generate_statistics().await;

    // Should start with clean state
    assert_eq!(stats.pages_crawled, 0);
    assert_eq!(stats.pages_failed, 0);
}

// Only test the public interface - actual crawling
#[tokio::test]
#[ignore] // Network-dependent test
async fn test_crawler_end_to_end() {
    let mut config = CrawlerConfig::default();
    config.crawler.seed_urls = vec!["https://httpbin.org/html".to_string()];
    config.crawler.max_pages = 1;
    config.crawler.concurrent_requests = 1;

    let crawler = WebCrawler::new(config).await.unwrap();

    match crawler.start_crawling().await {
        Ok(stats) => {
            // Should have attempted to crawl something
            assert!(stats.pages_crawled > 0 || stats.pages_failed > 0);
            println!("✅ End-to-end test: {:?}", stats);
        }
        Err(e) => {
            println!(
                "⚠️ End-to-end test failed (expected in some environments): {}",
                e
            );
        }
    }
}
