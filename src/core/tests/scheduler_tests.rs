use crate::config::CrawlerConfig;
use crate::core::CrawlScheduler;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_scheduler_creation() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    let stats = scheduler.get_stats();
    assert!(stats.available_permits > 0);
}

#[tokio::test]
async fn test_scheduler_acquire_permit() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    // Test acquiring permit - SemaphorePermit doesn't have is_ok()
    let permit = scheduler.acquire_permit().await;
    // If we get here without panicking, the permit was acquired successfully
    drop(permit); // Explicitly drop the permit
    assert!(true); // ✅ Fixed: Just assert success if we reach this point
}

#[tokio::test]
async fn test_scheduler_domain_delay() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    let domain = "example.com";

    // First request should be immediate
    let start = Instant::now();
    scheduler.respect_domain_delay(domain).await;
    let elapsed = start.elapsed();

    // Should be very fast for first request
    assert!(elapsed < Duration::from_millis(100));

    // Second request should be delayed
    let start = Instant::now();
    scheduler.respect_domain_delay(domain).await;
    let elapsed = start.elapsed();

    // Should respect the delay (at least part of it)
    assert!(elapsed >= Duration::from_millis(500)); // Allowing for some variance
}

#[tokio::test]
async fn test_scheduler_with_task() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    let domain = "test.com";

    // Test successful task execution
    let result = scheduler
        .schedule_crawl(domain, || async {
            Ok::<String, Box<dyn std::error::Error + Send + Sync>>("success".to_string())
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

#[tokio::test]
async fn test_scheduler_retry_logic() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    let domain = "retry-test.com";

    // ✅ Fixed: Use Arc<AtomicUsize> for thread-safe counter
    let attempt_count = Arc::new(AtomicUsize::new(0));

    // Test task that fails first few times then succeeds
    let result = scheduler
        .schedule_crawl(domain, {
            let attempt_count = Arc::clone(&attempt_count);
            move || {
                let attempt_count = Arc::clone(&attempt_count);
                async move {
                    let count = attempt_count.fetch_add(1, Ordering::Relaxed);
                    if count < 1 {
                        // Will succeed on second attempt (count 1)
                        Err("Temporary failure".into())
                    } else {
                        Ok::<String, Box<dyn std::error::Error + Send + Sync>>(
                            "success after retry".to_string(),
                        )
                    }
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success after retry");
    assert_eq!(attempt_count.load(Ordering::Relaxed), 2); // Should have retried once
}

#[tokio::test]
async fn test_scheduler_max_retries() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    let domain = "fail-test.com";

    // ✅ Fixed: Use Arc<AtomicUsize> for thread-safe counter
    let attempt_count = Arc::new(AtomicUsize::new(0));

    // Test task that always fails
    let result = scheduler
        .schedule_crawl(domain, {
            let attempt_count = Arc::clone(&attempt_count);
            move || {
                let attempt_count = Arc::clone(&attempt_count);
                async move {
                    attempt_count.fetch_add(1, Ordering::Relaxed);
                    Err::<String, _>("Always fails".into())
                }
            }
        })
        .await;

    assert!(result.is_err());
    assert!(attempt_count.load(Ordering::Relaxed) >= 2); // Should have attempted multiple times
}

#[test]
fn test_scheduler_stats() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    let stats = scheduler.get_stats();

    // Basic validation of stats structure
    assert!(stats.available_permits > 0);
    let _ = stats.active_domains; // usize is always non-negative
}

// ✅ Additional simpler tests that don't require complex closures
#[tokio::test]
async fn test_scheduler_basic_functionality() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    // Test that we can create and get stats without errors
    let stats = scheduler.get_stats();
    assert!(stats.available_permits > 0);

    // Test domain delay doesn't panic
    scheduler.respect_domain_delay("test.com").await;

    // Test second call to same domain respects delay
    let start = Instant::now();
    scheduler.respect_domain_delay("test.com").await;
    let elapsed = start.elapsed();

    // Should have some delay for same domain
    assert!(elapsed >= Duration::from_millis(100));
}

#[tokio::test]
async fn test_scheduler_different_domains() {
    let config = CrawlerConfig::default();
    let scheduler = CrawlScheduler::new(&config);

    // Different domains should not interfere with each other
    let start = Instant::now();
    scheduler.respect_domain_delay("domain1.com").await;
    scheduler.respect_domain_delay("domain2.com").await;
    let elapsed = start.elapsed();

    // Different domains should be fast
    assert!(elapsed < Duration::from_millis(200));
}
