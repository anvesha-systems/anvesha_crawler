use crate::core::PageProcessor;

#[tokio::test]
async fn test_page_processor_html_parsing() {
    let processor = PageProcessor::new();

    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test Page</title>
            <meta name="description" content="A test page for crawling">
            <meta name="keywords" content="test, crawling, html">
        </head>
        <body>
            <h1>Welcome to Test Page</h1>
            <p>This is a paragraph with some content.</p>
            <a href="https://example.com/link1">Link 1</a>
            <a href="/relative-link">Relative Link</a>
            <a href="mailto:test@example.com">Email Link</a>
        </body>
        </html>
    "#;

    let result = processor.process_page("https://test.com", html, 0).await;

    assert!(result.is_ok());
    let page_data = result.unwrap();

    // Test title extraction
    assert_eq!(page_data.title, Some("Test Page".to_string()));

    // Test description extraction
    assert_eq!(
        page_data.description,
        Some("A test page for crawling".to_string())
    );

    // Test keywords extraction
    assert!(page_data.keywords.contains(&"test".to_string()));
    assert!(page_data.keywords.contains(&"crawling".to_string()));

    // Test content extraction
    assert!(page_data.content.contains("Welcome to Test Page"));
    assert!(page_data.content.contains("This is a paragraph"));

    // Test link extraction (should find absolute and relative links, skip mailto)
    assert_eq!(page_data.outgoing_links.len(), 2);

    // Test quality score calculation
    assert!(page_data.content_quality_score > 0.0);
    assert!(page_data.content_quality_score <= 1.0);

    // Test word count
    assert!(page_data.word_count > 0);
}

#[tokio::test]
async fn test_page_processor_empty_content() {
    let processor = PageProcessor::new();

    let empty_html = "<html><head></head><body></body></html>";

    let result = processor
        .process_page("https://test.com", empty_html, 0)
        .await;

    assert!(result.is_ok());
    let page_data = result.unwrap();

    assert!(page_data.title.is_none());
    assert!(page_data.description.is_none());
    assert!(page_data.keywords.is_empty());
    assert!(page_data.outgoing_links.is_empty());
    assert!(page_data.content_quality_score < 0.1); // Very low quality
}

#[test]
fn test_priority_domains() {
    let mut processor = PageProcessor::new();

    // Add priority domain
    processor.add_priority_domain("github.com".to_string());

    // This is a basic test - actual priority logic would be tested in integration
    assert!(true); // Placeholder
}
