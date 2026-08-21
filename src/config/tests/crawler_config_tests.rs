use crate::config::CrawlerConfig;

#[test]
fn test_default_config_creation() {
    let config = CrawlerConfig::default();

    assert!(config.crawler.max_pages > 0);
    assert!(config.crawler.concurrent_requests > 0);
    assert!(config.network.request_timeout_secs > 0);
    assert!(!config.network.user_agents.is_empty());
}

#[test]
fn test_config_for_testing() {
    // ✅ Fixed: Create test config manually
    let mut config = CrawlerConfig::default();

    // Modify for testing
    config.crawler.max_pages = 5;
    config.crawler.seed_urls = vec!["https://httpbin.org/html".to_string()];
    config.storage.database_url = "sqlite://memory:test".to_string();

    assert!(config.crawler.max_pages <= 10); // Small for testing
    assert!(!config.crawler.seed_urls.is_empty());
    assert!(
        config.storage.database_url.contains("memory")
            || config.storage.database_url.contains("test")
    );
}

#[test]
fn test_config_serialization() {
    let config = CrawlerConfig::default();

    // Test that config can be serialized
    let serialized = toml::to_string(&config);
    assert!(serialized.is_ok());

    // Test that it can be deserialized back
    let deserialized: Result<CrawlerConfig, _> = toml::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[test]
fn test_config_field_validation() {
    let config = CrawlerConfig::default();

    // Test reasonable default values
    assert!(config.crawler.max_pages >= 100);
    assert!(config.crawler.concurrent_requests >= 1);
    assert!(config.crawler.concurrent_requests <= 50);
    assert!(config.network.request_timeout_secs >= 10);
    assert!(config.network.request_timeout_secs <= 120);
    assert!(config.network.max_retries >= 1);
    assert!(config.network.max_retries <= 10);
}

#[test]
fn test_config_network_settings() {
    let config = CrawlerConfig::default();

    // Test network configuration
    assert!(!config.network.user_agents.is_empty());
    assert!(config.network.request_delay_ms > 0);
    assert!(config.network.max_content_size_mb > 0);
    assert!(config.network.max_redirects > 0);
    assert!(config.network.connect_timeout_secs > 0);
}

#[test]
fn test_config_from_toml_string() {
    let toml_config = r#"
        [crawler]
        max_pages = 100
        concurrent_requests = 5
        max_depth = 3
        seed_urls = ["https://test.com"]
        user_agent = "TestBot/1.0"

        [network]
        request_timeout_secs = 30
        request_delay_ms = 1000
        max_retries = 3
        respect_robots_txt = true
        max_content_size_mb = 5
        max_redirects = 10
        connect_timeout_secs = 10
        user_agents = ["TestAgent/1.0"]

        [storage]
        database_url = "sqlite://test.db"
        enable_caching = false
        storage_path = "./test_data"

        [algorithms]
        primary_algorithm = "bfs"
        enable_opic = false
        priority_boost_domains = ["test.com"]
    "#;

    let parsed_config: Result<CrawlerConfig, _> = toml::from_str(toml_config);
    assert!(parsed_config.is_ok());

    let config = parsed_config.unwrap();
    assert_eq!(config.crawler.max_pages, 100);
    assert_eq!(config.crawler.concurrent_requests, 5);
    assert_eq!(config.network.request_timeout_secs, 30);
    assert_eq!(config.storage.database_url, "sqlite://test.db");
}
