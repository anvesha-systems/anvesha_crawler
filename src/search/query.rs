use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::{Document, Index, IndexReader, ReloadPolicy};
use tracing::info;

use super::filters::{SearchFilter, SortBy};
use super::schema::SearchSchema;
use super::snippets::SnippetGenerator;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: Option<String>,
    pub domain: String,
    pub quality_score: f64,
    pub score: f32,
    pub pagerank: f64,
    pub tfidf: f64,
    pub crawled_at: Option<String>,
    pub snippet: Option<String>,
}

pub struct SearchQuery {
    index: Index,
    reader: IndexReader,
    search_schema: SearchSchema,
}

impl SearchQuery {
    pub fn new(index_path: &Path) -> tantivy::Result<Self> {
        let index = Index::open_in_dir(index_path)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let search_schema = SearchSchema::build();

        Ok(Self {
            index,
            reader,
            search_schema,
        })
    }

    pub fn search(&self, query_str: &str, limit: usize) -> tantivy::Result<Vec<SearchResult>> {
        self.search_with_filters(
            query_str,
            limit,
            SearchFilter::new(),
            SortBy::Relevance,
            0,
            false,
            false,
        )
    }

    pub fn search_with_filters(
        &self,
        query_str: &str,
        limit: usize,
        filters: SearchFilter,
        sort_by: SortBy,
        offset: usize,
        generate_snippets: bool,
        highlight: bool,
    ) -> tantivy::Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        // Create query parser
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.search_schema.title_field,
                self.search_schema.content_field,
                self.search_schema.url_field,
            ],
        );

        let query = query_parser.parse_query(query_str)?;

        // Fetch more results for filtering
        let fetch_limit = if filters.has_filters() {
            (limit + offset) * 10
        } else {
            limit + offset
        };

        // Search and get top results
        let top_docs = searcher.search(&query, &TopDocs::with_limit(fetch_limit))?;

        // Prepare snippet generator
        let snippet_gen = SnippetGenerator::new();
        let query_terms = SnippetGenerator::extract_terms(query_str);

        let mut results = Vec::new();
        for (tantivy_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let url = retrieved_doc
                .get_first(self.search_schema.url_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = retrieved_doc
                .get_first(self.search_schema.title_field)
                .and_then(|v| v.as_str())
                .map(String::from);

            let domain = retrieved_doc
                .get_first(self.search_schema.domain_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let quality_score = retrieved_doc
                .get_first(self.search_schema.quality_field)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            // 🔥 NEW: Extract PageRank from index
            let pagerank = retrieved_doc
                .get_first(self.search_schema.pagerank_field)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            // Generate snippet if requested
            let snippet = if generate_snippets {
                let content_opt = retrieved_doc
                    .get_first(self.search_schema.content_field)
                    .and_then(|v| v.as_str());

                match content_opt {
                    Some(content) => {
                        let snippet_text = snippet_gen.generate(content, &query_terms, highlight);
                        Some(snippet_text)
                    }
                    None => None,
                }
            } else {
                None
            };

            // Apply filters
            if let Some(ref filter_domain) = filters.domain {
                if &domain != filter_domain {
                    continue;
                }
            }

            if let Some(min_q) = filters.min_quality {
                if quality_score < min_q {
                    continue;
                }
            }

            if let Some(max_q) = filters.max_quality {
                if quality_score > max_q {
                    continue;
                }
            }

            // NEW: Calculate combined score
            // Formula: 60% relevance + 40% PageRank (scaled)
            // Note: PageRank is typically 0.0-0.2, so we scale it by 100
            let pagerank_scaled = pagerank * 100.0;

            let tfidf = retrieved_doc
                .get_first(self.search_schema.tfidf_field)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let tfidf_sealed = tfidf * 100.0;

            let penalty = SearchQuery::utility_penalty(&url);

            let combined_score =
                ((tantivy_score as f64 * 0.6) + (pagerank_scaled * 0.25) + (tfidf_sealed * 0.15))
                    * penalty;

            results.push(SearchResult {
                url,
                title,
                domain,
                quality_score,
                score: combined_score as f32, // Use combined score
                pagerank,                     // Store PageRank separately
                tfidf,
                crawled_at: None,
                snippet,
            });
        }

        // Apply sorting BEFORE pagination
        match sort_by {
            SortBy::Relevance => {
                // Sort by combined score (already calculated above)
                results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::Quality => {
                // Sort by quality_score descending
                results.sort_by(|a, b| {
                    b.quality_score
                        .partial_cmp(&a.quality_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::PageRank => {
                results.sort_by(|a, b| {
                    b.pagerank
                        .partial_cmp(&a.pagerank)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            SortBy::TfIdf => {
                results.sort_by(|a, b| {
                    b.tfidf
                        .partial_cmp(&a.tfidf)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            SortBy::Date => {
                // TODO: Sort by crawled_at when we add it to index
            }
        }

        // Apply pagination AFTER sorting
        let paginated: Vec<SearchResult> = results.into_iter().skip(offset).take(limit).collect();

        info!(result_count = paginated.len(), "search completed");
        Ok(paginated)
    }

    fn utility_penalty(url: &str) -> f64 {
        if url.contains("action=edit")
            || url.contains("action=history")
            || url.contains("/Special:")
        {
            0.85 // stronger penalty
        } else if url.contains("#") {
            0.95 // mild penalty for section anchors
        } else {
            1.0
        }
    }

    pub fn search_by_domain(
        &self,
        query_str: &str,
        domain: &str,
        limit: usize,
    ) -> tantivy::Result<Vec<SearchResult>> {
        let filters = SearchFilter::new().with_domain(domain.to_string());
        self.search_with_filters(
            query_str,
            limit,
            filters,
            SortBy::Relevance,
            0,
            false,
            false,
        )
    }
}
