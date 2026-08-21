use crate::models::{CrawlUrl, PageData};
use chrono::Utc;

#[test]
fn test_page_data_creation() {
    let page_data = PageData {
        url: "https://example.com".to_string(),
        title: Some("Test Page".to_string()),
        description: Some("A test page".to_string()),
        keywords: vec!["test".to_string(), "page".to_string()],
        content: "This is test content with multiple words for testing.".to_string(),
        outgoing_links: vec![],
        word_count: 10,
        content_quality_score: 0.75,
        crawled_at: Utc::now(),
        depth: 1,
    };

    assert_eq!(page_data.url, "https://example.com");
    assert_eq!(page_data.title, Some("Test Page".to_string()));
    assert_eq!(page_data.description, Some("A test page".to_string()));
    assert_eq!(page_data.keywords.len(), 2);
    assert!(page_data.keywords.contains(&"test".to_string()));
    assert_eq!(page_data.word_count, 10);
    assert_eq!(page_data.content_quality_score, 0.75);
    assert_eq!(page_data.depth, 1);
}

#[test]
fn test_page_data_with_links() {
    let outgoing_links = vec![
        CrawlUrl {
            url: "https://example.com/link1".to_string(),
            priority: 5.0,
            depth: 2,
            discovered_at: Utc::now().timestamp() as u64,
        },
        CrawlUrl {
            url: "https://example.com/link2".to_string(),
            priority: 3.0,
            depth: 2,
            discovered_at: Utc::now().timestamp() as u64,
        },
    ];

    let page_data = PageData {
        url: "https://example.com".to_string(),
        title: Some("Page with Links".to_string()),
        description: None,
        keywords: vec![],
        content: "Content with outgoing links".to_string(),
        outgoing_links: outgoing_links.clone(),
        word_count: 4,
        content_quality_score: 0.5,
        crawled_at: Utc::now(),
        depth: 1,
    };

    assert_eq!(page_data.outgoing_links.len(), 2);
    assert_eq!(page_data.outgoing_links[0].url, "https://example.com/link1");
    assert_eq!(page_data.outgoing_links[1].url, "https://example.com/link2");
    assert_eq!(page_data.outgoing_links[0].priority, 5.0);
    assert_eq!(page_data.outgoing_links[1].priority, 3.0);
}

#[test]
fn test_page_data_empty_content() {
    let page_data = PageData {
        url: "https://example.com/empty".to_string(),
        title: None,
        description: None,
        keywords: vec![],
        content: String::new(),
        outgoing_links: vec![],
        word_count: 0,
        content_quality_score: 0.0,
        crawled_at: Utc::now(),
        depth: 0,
    };

    assert!(page_data.title.is_none());
    assert!(page_data.description.is_none());
    assert!(page_data.keywords.is_empty());
    assert!(page_data.content.is_empty());
    assert!(page_data.outgoing_links.is_empty());
    assert_eq!(page_data.word_count, 0);
    assert_eq!(page_data.content_quality_score, 0.0);
}

#[test]
fn test_page_data_quality_score_range() {
    // Test that quality score is within valid range
    let mut page_data = PageData {
        url: "https://example.com".to_string(),
        title: Some("Test".to_string()),
        description: None,
        keywords: vec![],
        content: "Test content".to_string(),
        outgoing_links: vec![],
        word_count: 2,
        content_quality_score: 1.5, // Invalid: > 1.0
        crawled_at: Utc::now(),
        depth: 0,
    };

    // In a real implementation, you might have validation
    // For now, just test the structure
    assert!(page_data.content_quality_score > 0.0);

    page_data.content_quality_score = -0.5; // Invalid: < 0.0
    // Again, in real implementation you'd validate this
    assert!(page_data.content_quality_score != 0.0);
}

#[test]
fn test_page_data_serialization() {
    let page_data = PageData {
        url: "https://example.com".to_string(),
        title: Some("Serialization Test".to_string()),
        description: Some("Testing serialization".to_string()),
        keywords: vec!["test".to_string(), "serialization".to_string()],
        content: "Test content for serialization".to_string(),
        outgoing_links: vec![],
        word_count: 5,
        content_quality_score: 0.8,
        crawled_at: Utc::now(),
        depth: 1,
    };

    // Test JSON serialization if PageData derives Serialize
    // let json = serde_json::to_string(&page_data);
    // assert!(json.is_ok());

    // For now, just test basic field access
    assert!(!page_data.url.is_empty());
    assert!(page_data.title.is_some());
    assert!(!page_data.keywords.is_empty());
}

#[test]
fn test_page_data_with_large_content() {
    let large_content = "word ".repeat(1000); // 1000 words

    let page_data = PageData {
        url: "https://example.com/large".to_string(),
        title: Some("Large Content Page".to_string()),
        description: Some("Page with lots of content".to_string()),
        keywords: vec!["large".to_string(), "content".to_string()],
        content: large_content.clone(),
        outgoing_links: vec![],
        word_count: 1000,
        content_quality_score: 0.9,
        crawled_at: Utc::now(),
        depth: 2,
    };

    assert_eq!(page_data.content.len(), large_content.len());
    assert_eq!(page_data.word_count, 1000);
    assert!(page_data.content_quality_score > 0.8);
}
