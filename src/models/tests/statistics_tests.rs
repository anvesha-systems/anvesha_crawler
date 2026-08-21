use crate::models::CrawlStatistics;
use std::time::Duration;

#[test]
fn test_crawl_statistics_creation() {
    let stats = CrawlStatistics {
        pages_crawled: 100,
        pages_failed: 5,
        urls_discovered: 500,
        urls_in_queue: 25,
        elapsed_time: Duration::from_secs(300), // 5 minutes
        crawl_rate: 0.33,                       // pages per second
    };

    assert_eq!(stats.pages_crawled, 100);
    assert_eq!(stats.pages_failed, 5);
    assert_eq!(stats.urls_discovered, 500);
    assert_eq!(stats.urls_in_queue, 25);
    assert_eq!(stats.elapsed_time, Duration::from_secs(300));
    assert_eq!(stats.crawl_rate, 0.33);
}

#[test]
fn test_crawl_statistics_initial_state() {
    let stats = CrawlStatistics {
        pages_crawled: 0,
        pages_failed: 0,
        urls_discovered: 0,
        urls_in_queue: 0,
        elapsed_time: Duration::from_secs(0),
        crawl_rate: 0.0,
    };

    assert_eq!(stats.pages_crawled, 0);
    assert_eq!(stats.pages_failed, 0);
    assert_eq!(stats.urls_discovered, 0);
    assert_eq!(stats.urls_in_queue, 0);
    assert_eq!(stats.elapsed_time, Duration::ZERO);
    assert_eq!(stats.crawl_rate, 0.0);
}

#[test]
fn test_crawl_statistics_calculations() {
    let stats = CrawlStatistics {
        pages_crawled: 50,
        pages_failed: 10,
        urls_discovered: 200,
        urls_in_queue: 150,
        elapsed_time: Duration::from_secs(100),
        crawl_rate: 0.5, // 50 pages / 100 seconds
    };

    // Test total pages attempted
    let total_attempted = stats.pages_crawled + stats.pages_failed;
    assert_eq!(total_attempted, 60);

    // Test success rate
    let success_rate = stats.pages_crawled as f64 / total_attempted as f64;
    assert!((success_rate - 0.8333).abs() < 0.001); // ~83.33%

    // Test that crawl rate makes sense
    let expected_rate = stats.pages_crawled as f64 / stats.elapsed_time.as_secs_f64();
    assert!((stats.crawl_rate - expected_rate).abs() < 0.001);
}

#[test]
fn test_crawl_statistics_edge_cases() {
    // Test with zero elapsed time
    let stats_zero_time = CrawlStatistics {
        pages_crawled: 10,
        pages_failed: 0,
        urls_discovered: 50,
        urls_in_queue: 40,
        elapsed_time: Duration::ZERO,
        crawl_rate: f64::INFINITY, // or handle this case specially
    };

    assert!(stats_zero_time.crawl_rate.is_infinite() || stats_zero_time.crawl_rate.is_nan());

    // Test with very long elapsed time
    let stats_long_time = CrawlStatistics {
        pages_crawled: 1,
        pages_failed: 0,
        urls_discovered: 10,
        urls_in_queue: 9,
        elapsed_time: Duration::from_secs(3600), // 1 hour
        crawl_rate: 1.0 / 3600.0,                // Very slow rate
    };

    assert!(stats_long_time.crawl_rate < 0.001);
    assert!(stats_long_time.crawl_rate > 0.0);
}

#[test]
fn test_crawl_statistics_realistic_scenario() {
    // Simulate a realistic crawling session
    let stats = CrawlStatistics {
        pages_crawled: 1500,
        pages_failed: 150,
        urls_discovered: 8000,
        urls_in_queue: 2500,
        elapsed_time: Duration::from_secs(3600), // 1 hour
        crawl_rate: 1500.0 / 3600.0,             // ~0.42 pages/second
    };

    // Validate realistic ranges
    assert!(stats.pages_crawled > 0);
    assert!(stats.pages_failed < stats.pages_crawled); // More success than failure
    assert!(stats.urls_discovered > stats.pages_crawled); // Found more URLs than crawled
    assert!(stats.urls_in_queue < stats.urls_discovered); // Some URLs already processed
    assert!(stats.crawl_rate > 0.0 && stats.crawl_rate < 10.0); // Reasonable rate
}

#[test]
fn test_crawl_statistics_debug_format() {
    let stats = CrawlStatistics {
        pages_crawled: 42,
        pages_failed: 3,
        urls_discovered: 200,
        urls_in_queue: 155,
        elapsed_time: Duration::from_secs(120),
        crawl_rate: 0.35,
    };

    // Test that Debug formatting works (if CrawlStatistics derives Debug)
    let debug_str = format!("{:?}", stats);
    assert!(debug_str.contains("pages_crawled"));
    assert!(debug_str.contains("42"));
}

#[test]
fn test_crawl_statistics_performance_metrics() {
    let stats = CrawlStatistics {
        pages_crawled: 100,
        pages_failed: 20,
        urls_discovered: 600,
        urls_in_queue: 380,
        elapsed_time: Duration::from_secs(200),
        crawl_rate: 0.5,
    };

    // Calculate various performance metrics
    let total_pages = stats.pages_crawled + stats.pages_failed;
    let success_percentage = (stats.pages_crawled as f64 / total_pages as f64) * 100.0;
    let discovery_rate = stats.urls_discovered as f64 / stats.pages_crawled as f64;
    let queue_utilization = stats.urls_in_queue as f64 / stats.urls_discovered as f64;

    assert_eq!(total_pages, 120);
    assert!((success_percentage - 83.33).abs() < 0.1); // ~83.33%
    assert_eq!(discovery_rate, 6.0); // 6 URLs per page crawled
    assert!((queue_utilization - 0.633).abs() < 0.001); // ~63.3% still in queue
}
