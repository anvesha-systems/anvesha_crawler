/// Process downloaded pages, extracts content and links
use crate::models::{CrawlUrl, PageData};
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::{debug, error};
use url::Url;

/// Processes HTML pages and extracts useful information
pub struct PageProcessor {
    /// Maximum number of links to extract per page
    max_links_per_page: usize,

    /// Domains to prioritize for link extraction
    priority_domains: HashSet<String>,

    /// File extensions to ignore
    ignored_extensions: HashSet<String>,
}

impl PageProcessor {
    pub fn new() -> Self {
        let mut ignored_extensions = HashSet::new();
        ignored_extensions.extend(
            [
                "jpg", "jpeg", "png", "gif", "pdf", "doc", "docx", "zip", "tar", "gz", "mp3",
                "mp4", "avi",
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        Self {
            max_links_per_page: 1000,
            priority_domains: HashSet::new(),
            ignored_extensions,
        }
    }

    /// Process HTML content and extract page data
    pub async fn process_page(
        &self,
        url: &str,
        html_content: &str,
        depth: u32,
    ) -> Result<PageData, ProcessorError> {
        let document = Html::parse_document(html_content);

        // Extract basic page information
        let title = self.extract_title(&document);
        let description = self.extract_description(&document);
        let keywords = self.extract_keywords(&document);
        let text_content = self.extract_text_content(&document);

        // Extract outgoing links
        let outgoing_links = self.extract_links(&document, url, depth + 1)?;

        // Calculate content metrics
        let word_count = text_content.split_whitespace().count();
        let content_quality_score = self.calculate_content_quality(&text_content, &title);

        Ok(PageData {
            url: url.to_string(),
            title,
            description,
            keywords,
            content: text_content,
            outgoing_links,
            word_count,
            content_quality_score,
            crawled_at: chrono::Utc::now(),
            depth,
        })
    }

    /// Extract page title
    fn extract_title(&self, document: &Html) -> Option<String> {
        let title_selector = Selector::parse("title").ok()?; // Fixed: Ok() -> ok()

        document
            .select(&title_selector)
            .next()
            .map(|element| element.text().collect::<String>().trim().to_string())
            .filter(|title| !title.is_empty())
    }

    /// Extract meta description
    fn extract_description(&self, document: &Html) -> Option<String> {
        let meta_selector = Selector::parse("meta[name='description']").ok()?;

        document
            .select(&meta_selector)
            .next()
            .and_then(|element| element.value().attr("content"))
            .map(|content| content.trim().to_string())
            .filter(|description| !description.is_empty())
    }

    /// Extract meta keywords
    fn extract_keywords(&self, document: &Html) -> Vec<String> {
        let meta_selector = Selector::parse("meta[name='keywords']").unwrap();

        document
            .select(&meta_selector)
            .next()
            .and_then(|element| element.value().attr("content"))
            .map(|content| {
                content
                    .split(',')
                    .map(|keyword| keyword.trim().to_string())
                    .filter(|keyword| !keyword.is_empty())
                    .collect()
            })
            .unwrap_or_else(Vec::new)
    }

    /// Extract main text content
    fn extract_text_content(&self, document: &Html) -> String {
        // Remove script and style elements
        let content_selectors = [
            "p", "h1", "h2", "h3", "h4", "h5", "h6", "article", "main", "section", "div",
        ];

        let mut text_parts = Vec::new();

        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<String>();
                    let clean_text = text.trim();
                    if !clean_text.is_empty() && clean_text.len() > 10 {
                        text_parts.push(clean_text.to_string());
                    }
                }
            }
        }
        text_parts.join(" ")
    }

    /// Extract outgoing links from the page
    fn extract_links(
        &self,
        document: &Html,
        base_url: &str,
        next_depth: u32,
    ) -> Result<Vec<CrawlUrl>, ProcessorError> {
        let link_selector =
            Selector::parse("a[href]").map_err(|_| ProcessorError::SelectorParseError)?;

        let base_url_parsed = Url::parse(base_url).map_err(|_| ProcessorError::InvalidBaseUrl)?;

        let mut links = Vec::new();
        let mut link_count = 0; // Fixed: line_count -> link_count

        for element in document.select(&link_selector) {
            if link_count >= self.max_links_per_page {
                // Fixed: line_count -> link_count
                break;
            }

            if let Some(href) = element.value().attr("href") {
                match self.resolve_and_validate_url(&base_url_parsed, href, next_depth) {
                    Ok(Some(crawl_url)) => {
                        links.push(crawl_url);
                        link_count += 1;
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        debug!("Error resolving link: {} : {}", href, e);
                        continue;
                    }
                }
            }
        }
        Ok(links)
    }

    /// Resolve relative URLs and validate
    fn resolve_and_validate_url(
        &self,
        base_url: &Url, // Fixed: Changed parameter type from &str to &Url
        href: &str,
        depth: u32, // Fixed: Changed parameter name from next_depth to depth for consistency
    ) -> Result<Option<CrawlUrl>, ProcessorError> {
        // Skip obvious non-web links
        if href.starts_with("mailto:")
            || href.starts_with("tel:")
            || href.starts_with("javascript:")
        {
            return Ok(None);
        }

        // Resolve relative URL
        let absolute_url = base_url
            .join(href)
            .map_err(|_| ProcessorError::UrlResolutionError)?; // Fixed: URLResolutionError -> UrlResolutionError

        let url_str = absolute_url.to_string();

        // Check for ignored file extensions
        if let Some(extension) = self.get_file_extension(&url_str) {
            // Fixed: extensions -> extension
            if self.ignored_extensions.contains(&extension.to_lowercase()) {
                return Ok(None);
            }
        }

        // Only crawl HTTP/HTTPS URLs
        if !matches!(absolute_url.scheme(), "http" | "https") {
            return Ok(None);
        }

        // Calculate priority based on domain and other factors
        let priority = self.calculate_link_priority(&absolute_url, depth);

        Ok(Some(CrawlUrl {
            url: url_str,
            priority,
            depth: depth,
            discovered_at: chrono::Utc::now().timestamp() as u64, // Fixed: discoverd_at -> discovered_at
        }))
    }

    /// Calculate content quality score (0.0 to 1.0)
    fn calculate_content_quality(&self, content: &str, title: &Option<String>) -> f64 {
        let mut score = 0.0;

        // Length factor (optimal around 500-2000 words)
        let word_count = content.split_whitespace().count();
        let length_score = match word_count {
            0..=50 => 0.1,
            51..=200 => 0.5,
            201..=500 => 0.8,
            501..=2000 => 1.0,
            2001..=5000 => 0.9, // Fixed: Removed underscore from 0.9_
            _ => 0.7,
        };

        score += length_score * 0.4;

        // Title presence
        if title.is_some() {
            score += 0.2;
        }

        // Content diversity (simple heuristic)
        let binding = content.to_lowercase();

        let unique_words: HashSet<_> = binding.split_whitespace().collect(); // Fixed: count() -> collect()

        let diversity_score = if word_count > 0 {
            (unique_words.len() as f64 / word_count as f64).min(1.0)
        } else {
            0.0
        };

        score += diversity_score * 0.4;

        score.min(1.0)
    }

    /// Calculate link priority
    fn calculate_link_priority(&self, url: &Url, depth: u32) -> f64 {
        let mut priority = 1.0 / (depth as f64 + 1.0);

        // Domain priority boost
        if let Some(host) = url.host_str() {
            for priority_domain in &self.priority_domains {
                if host.contains(priority_domain) {
                    priority *= 2.0;
                    break;
                }
            }
        }

        // Path-based priority adjustments
        let path = url.path();
        if path.contains("/article/") || path.contains("/post/") || path.contains("/blog/") {
            priority *= 1.5;
        }

        priority
    }

    /// Get file extension from URL
    fn get_file_extension(&self, url: &str) -> Option<String> {
        url.split('?')
            .next()? // Remove query parameters
            .split('#')
            .next()? // Remove fragment
            .split('/')
            .last()? // Get filename
            .split('.')
            .last() // Get extension
            .filter(|ext| !ext.is_empty()) // Make sure it's not empty
            .map(|ext| ext.to_string()) // Convert to owned String
    }

    /// Add priority domain
    pub fn add_priority_domain(&mut self, domain: String) {
        self.priority_domains.insert(domain);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("Failed to parse CSS selector")]
    SelectorParseError,

    #[error("Invalid base URL")]
    InvalidBaseUrl,

    #[error("URL resolution error")]
    UrlResolutionError, // Fixed: URLResolutionError -> UrlResolutionError (consistent naming)
}
