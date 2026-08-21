use crate::models::PageData;
use crate::storage::{Result, SearchResult, StorageError, StoredPage};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tantivy::schema::{NumericOptions, Value};
use tantivy::tokenizer::{
    Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, TextAnalyzer,
};
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions},
};
use tracing::{debug, info};

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    schema: Schema,
    // Common fields
    id_field: Field,
    url_field: Field,
    domain_field: Field,
    quality_field: Field,
    language_field: Field,

    // English fields
    title_en_field: Field,
    description_en_field: Field,
    content_en_field: Field,

    // Indian language fields
    title_hi_field: Field, // [translate:हिंदी] (Hindi)
    content_hi_field: Field,
    title_kn_field: Field, // [translate:ಕನ್ನಡ] (Kannada)
    content_kn_field: Field,
    title_ta_field: Field, // [translate:தமிழ்] (Tamil)
    content_ta_field: Field,
    title_te_field: Field, // [translate:తెలుగు] (Telugu)
    content_te_field: Field,
    title_ml_field: Field, // [translate:മലയാളം] (Malayalam)
    content_ml_field: Field,
    title_mr_field: Field, // [translate:मराठी] (Marathi)
    content_mr_field: Field,
}

impl SearchIndex {
    pub fn new(index_path: &Path) -> Result<Self> {
        info!(
            "Creating 6-language Indian search index at: {:?}",
            index_path
        );

        let mut schema_builder = tantivy::schema::SchemaBuilder::new();

        // Base configuration for all languages
        let base_searchable = |tokenizer: &str| {
            TextOptions::default().set_stored().set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer(tokenizer)
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
        };

        let text_raw_stored = TextOptions::default().set_stored().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("raw")
                .set_index_option(IndexRecordOption::WithFreqs),
        );

        // Numeric fields
        let num_stored = NumericOptions::default().set_stored();
        let num_fast_stored = NumericOptions::default().set_fast().set_stored();

        // Common fields
        let id_field = schema_builder.add_i64_field("id", num_fast_stored);
        let url_field = schema_builder.add_text_field("url", text_raw_stored.clone());
        let domain_field = schema_builder.add_text_field("domain", text_raw_stored.clone());
        let language_field = schema_builder.add_text_field("language", text_raw_stored);
        let quality_field = schema_builder.add_f64_field("quality", num_stored);

        // English fields
        let title_en_field = schema_builder.add_text_field("title_en", base_searchable("english"));
        let description_en_field =
            schema_builder.add_text_field("description_en", base_searchable("english"));
        let content_en_field =
            schema_builder.add_text_field("content_en", base_searchable("english"));

        // Indian language fields
        let title_hi_field = schema_builder.add_text_field("title_hi", base_searchable("hindi"));
        let content_hi_field =
            schema_builder.add_text_field("content_hi", base_searchable("hindi"));

        let title_kn_field = schema_builder.add_text_field("title_kn", base_searchable("kannada"));
        let content_kn_field =
            schema_builder.add_text_field("content_kn", base_searchable("kannada"));

        let title_ta_field = schema_builder.add_text_field("title_ta", base_searchable("tamil"));
        let content_ta_field =
            schema_builder.add_text_field("content_ta", base_searchable("tamil"));

        let title_te_field = schema_builder.add_text_field("title_te", base_searchable("telugu"));
        let content_te_field =
            schema_builder.add_text_field("content_te", base_searchable("telugu"));

        let title_ml_field =
            schema_builder.add_text_field("title_ml", base_searchable("malayalam"));
        let content_ml_field =
            schema_builder.add_text_field("content_ml", base_searchable("malayalam"));

        let title_mr_field = schema_builder.add_text_field("title_mr", base_searchable("marathi"));
        let content_mr_field =
            schema_builder.add_text_field("content_mr", base_searchable("marathi"));

        let schema = schema_builder.build();

        std::fs::create_dir_all(index_path)
            .map_err(|e| StorageError::SearchIndex(format!("Failed to create index dir: {}", e)))?;

        let index = Index::open_in_dir(index_path)
            .or_else(|_| Index::create_in_dir(index_path, schema.clone()))
            .map_err(|e| {
                StorageError::SearchIndex(format!("Failed to create/open index: {}", e))
            })?;

        // Register all 6 Indian language tokenizers
        Self::register_indian_tokenizers(&index);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| StorageError::SearchIndex(format!("Failed to build reader: {}", e)))?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| StorageError::SearchIndex(format!("Failed to create writer: {}", e)))?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            schema,
            id_field,
            url_field,
            domain_field,
            quality_field,
            language_field,
            title_en_field,
            description_en_field,
            content_en_field,
            title_hi_field,
            content_hi_field,
            title_kn_field,
            content_kn_field,
            title_ta_field,
            content_ta_field,
            title_te_field,
            content_te_field,
            title_ml_field,
            content_ml_field,
            title_mr_field,
            content_mr_field,
        })
    }

    // Register tokenizers for all 6 Indian languages + English
    fn register_indian_tokenizers(index: &Index) {
        // English tokenizer with stemming
        let english_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::English))
            .build();
        index.tokenizers().register("english", english_tokenizer);

        // Hindi tokenizer (Devanagari: U+0900-U+097F)
        let hindi_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(120)) // Longer for compound words
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("hindi", hindi_tokenizer);

        // Kannada tokenizer (U+0C80-U+0CFF) [web:31][web:33]
        let kannada_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(120))
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("kannada", kannada_tokenizer);

        // Tamil tokenizer (U+0B80-U+0BFF)
        let tamil_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(120))
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("tamil", tamil_tokenizer);

        // Telugu tokenizer (U+0C00-U+0C7F)
        let telugu_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(120))
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("telugu", telugu_tokenizer);

        // Malayalam tokenizer (U+0D00-U+0D7F) [web:30][web:32]
        let malayalam_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(120))
            .filter(LowerCaser)
            .build();
        index
            .tokenizers()
            .register("malayalam", malayalam_tokenizer);

        // Marathi tokenizer (Uses Devanagari like Hindi: U+0900-U+097F) [web:45]
        let marathi_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(120))
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("marathi", marathi_tokenizer);

        info!("Registered tokenizers for: English + 6 Indian languages");
    }

    // Advanced language detection using Unicode ranges [web:30][web:31]
    fn detect_content_language(&self, content: &str) -> String {
        let char_counts = content.chars().fold(
            [0u32; 7], // [english, hindi, kannada, tamil, telugu, malayalam, marathi]
            |mut counts, c| {
                match c as u32 {
                    // English (Basic Latin + Latin-1)
                    0x0000..=0x024F => counts[0] += 1,
                    // Hindi & Marathi (Devanagari: U+0900-U+097F) [web:45]
                    0x0900..=0x097F => {
                        // Further distinguish Hindi vs Marathi by common patterns
                        counts[1] += 1; // Default to Hindi
                        counts[6] += 1; // Also count for Marathi
                    }
                    // Kannada (U+0C80-U+0CFF) [web:31][web:33]
                    0x0C80..=0x0CFF => counts[2] += 1,
                    // Tamil (U+0B80-U+0BFF)
                    0x0B80..=0x0BFF => counts[3] += 1,
                    // Telugu (U+0C00-U+0C7F)
                    0x0C00..=0x0C7F => counts[4] += 1,
                    // Malayalam (U+0D00-U+0D7F) [web:30][web:32]
                    0x0D00..=0x0D7F => counts[5] += 1,
                    _ => {}
                }
                counts
            },
        );

        // Find the script with the highest character count - FIXED: Use owned values
        let max_idx = char_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, count)| *count) // Changed from &count to *count
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        // Return language code based on highest count
        match max_idx {
            1 => {
                // Distinguish Hindi vs Marathi
                if char_counts[6] > char_counts[1] / 2 {
                    // If Marathi count is significant, do additional checks
                    if content.contains("मराठी") || content.contains("महाराष्ट्र")
                    {
                        "mr".to_string() // Marathi
                    } else {
                        "hi".to_string() // Default to Hindi
                    }
                } else {
                    "hi".to_string()
                }
            }
            2 => "kn".to_string(), // Kannada
            3 => "ta".to_string(), // Tamil
            4 => "te".to_string(), // Telugu
            5 => "ml".to_string(), // Malayalam
            6 => "mr".to_string(), // Marathi
            _ => "en".to_string(), // Default to English
        }
    }

    // Language-aware indexing with proper field mapping
    pub fn index_page(&self, page_id: i64, page: &PageData) -> Result<()> {
        let detected_language = self.detect_content_language(&page.content);
        self.index_page_with_language(page_id, page, &detected_language)
    }

    pub fn index_page_with_language(
        &self,
        page_id: i64,
        page: &PageData,
        detected_language: &str,
    ) -> Result<()> {
        let mut doc = TantivyDocument::new(); // FIXED: Use Document::new() instead of default()

        // Common fields
        doc.add_i64(self.id_field, page_id);
        doc.add_text(self.url_field, &page.url);
        doc.add_text(self.language_field, detected_language);
        let domain = page.url.split('/').nth(2).unwrap_or("unknown");
        doc.add_text(self.domain_field, domain);
        doc.add_f64(self.quality_field, page.content_quality_score);

        // Index in language-specific fields
        match detected_language {
            "hi" => {
                // [translate:हिंदी] Hindi
                if let Some(title) = &page.title {
                    doc.add_text(self.title_hi_field, title);
                }
                doc.add_text(self.content_hi_field, &page.content);
                info!("Indexed Hindi content: {}", page.url);
            }
            "kn" => {
                // [translate:ಕನ್ನಡ] Kannada
                if let Some(title) = &page.title {
                    doc.add_text(self.title_kn_field, title);
                }
                doc.add_text(self.content_kn_field, &page.content);
                info!("Indexed Kannada content: {}", page.url);
            }
            "ta" => {
                // [translate:தமிழ்] Tamil
                if let Some(title) = &page.title {
                    doc.add_text(self.title_ta_field, title);
                }
                doc.add_text(self.content_ta_field, &page.content);
                info!("Indexed Tamil content: {}", page.url);
            }
            "te" => {
                // [translate:తెలుగు] Telugu
                if let Some(title) = &page.title {
                    doc.add_text(self.title_te_field, title);
                }
                doc.add_text(self.content_te_field, &page.content);
                info!("📝 Indexed Telugu content: {}", page.url);
            }
            "ml" => {
                // [translate:മലയാളം] Malayalam
                if let Some(title) = &page.title {
                    doc.add_text(self.title_ml_field, title);
                }
                doc.add_text(self.content_ml_field, &page.content);
                info!("📝 Indexed Malayalam content: {}", page.url);
            }
            "mr" => {
                // [translate:मराठी] Marathi
                if let Some(title) = &page.title {
                    doc.add_text(self.title_mr_field, title);
                }
                doc.add_text(self.content_mr_field, &page.content);
                info!("Indexed Marathi content: {}", page.url);
            }
            _ => {
                // English (default)
                if let Some(title) = &page.title {
                    doc.add_text(self.title_en_field, title);
                }
                if let Some(description) = &page.description {
                    doc.add_text(self.description_en_field, description);
                }
                doc.add_text(self.content_en_field, &page.content);
                info!("Indexed English content: {}", page.url);
            }
        }

        {
            let writer = self.writer.lock().unwrap();
            writer
                .add_document(doc)
                .map_err(|e| StorageError::SearchIndex(format!("Failed to add document: {}", e)))?;
        }

        debug!(
            "Indexed {} page: {} (ID: {})",
            detected_language, page.url, page_id
        );
        Ok(())
    }

    //  Smart multi-language search
    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<SearchResult>> {
        // Auto-detect query language and search appropriately
        let query_language = self.detect_content_language(query_str);
        self.search_with_language(query_str, Some(&query_language), limit, offset)
    }

    pub fn search_with_language(
        &self,
        query_str: &str,
        language: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        // 🔥 Select search fields based on language
        let search_fields = match language {
            Some("hi") => vec![self.title_hi_field, self.content_hi_field],
            Some("kn") => vec![self.title_kn_field, self.content_kn_field],
            Some("ta") => vec![self.title_ta_field, self.content_ta_field],
            Some("te") => vec![self.title_te_field, self.content_te_field],
            Some("ml") => vec![self.title_ml_field, self.content_ml_field],
            Some("mr") => vec![self.title_mr_field, self.content_mr_field],
            Some("en") => vec![
                self.title_en_field,
                self.description_en_field,
                self.content_en_field,
            ],
            None => {
                // Search across ALL languages
                vec![
                    self.title_en_field,
                    self.content_en_field,
                    self.title_hi_field,
                    self.content_hi_field,
                    self.title_kn_field,
                    self.content_kn_field,
                    self.title_ta_field,
                    self.content_ta_field,
                    self.title_te_field,
                    self.content_te_field,
                    self.title_ml_field,
                    self.content_ml_field,
                    self.title_mr_field,
                    self.content_mr_field,
                ]
            }
            _ => vec![
                self.title_en_field,
                self.description_en_field,
                self.content_en_field,
            ], // fallback
        };

        let query_parser = QueryParser::for_index(&self.index, search_fields);

        let mut final_query_str = query_str.to_string();

        // Add language filter if specified
        if let Some(lang) = language {
            final_query_str = format!("({}) AND language:{}", query_str, lang);
        }

        let query = query_parser
            .parse_query(&final_query_str)
            .map_err(|e| StorageError::SearchIndex(format!("Failed to parse query: {}", e)))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit + offset))
            .map_err(|e| StorageError::SearchIndex(format!("Failed to search: {}", e)))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs.into_iter().skip(offset) {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| StorageError::SearchIndex(format!("Failed to fetch doc: {}", e)))?;

            let id: i64 = retrieved_doc
                .get_first(self.id_field) // Option<CompactDocValue>
                .and_then(|v| v.as_i64()) // Option<i64>
                .unwrap_or(0);

            let url = retrieved_doc
                .get_first(self.url_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let detected_lang = retrieved_doc
                .get_first(self.language_field)
                .and_then(|v| v.as_str())
                .unwrap_or("en");

            // Get title from appropriate language field
            let title = match detected_lang {
                "hi" => retrieved_doc
                    .get_first(self.title_hi_field)
                    .and_then(|v| v.as_str()),
                "kn" => retrieved_doc
                    .get_first(self.title_kn_field)
                    .and_then(|v| v.as_str()),
                "ta" => retrieved_doc
                    .get_first(self.title_ta_field)
                    .and_then(|v| v.as_str()),
                "te" => retrieved_doc
                    .get_first(self.title_te_field)
                    .and_then(|v| v.as_str()),
                "ml" => retrieved_doc
                    .get_first(self.title_ml_field)
                    .and_then(|v| v.as_str()),
                "mr" => retrieved_doc
                    .get_first(self.title_mr_field)
                    .and_then(|v| v.as_str()),
                _ => retrieved_doc
                    .get_first(self.title_en_field)
                    .and_then(|v| v.as_str()),
            }
            .map(|s| s.to_string());

            let description = retrieved_doc
                .get_first(self.description_en_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let domain = retrieved_doc
                .get_first(self.domain_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let quality_score = retrieved_doc
                .get_first(self.quality_field)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let snippet = description.clone().unwrap_or_else(|| {
                if url.len() > 100 {
                    format!("{}...", &url[..100])
                } else {
                    url.clone()
                }
            });

            let stored_page = StoredPage {
                id,
                url: url.clone(),
                url_hash: String::new(),
                domain,
                title: title.clone(),
                description: description.clone(),
                content: String::new(),
                content_hash: String::new(),
                quality_score,
                word_count: 0,
                language: detected_lang.to_string(),
                crawl_depth: 0,
                crawled_at: chrono::Utc::now(),
                last_modified: None,
                status_code: 200,
                content_type: "text/html".to_string(),
                content_length: 0,
                pagerank: None,
                tfidf_score: None,
            };

            results.push(SearchResult::new(stored_page, score, snippet));
        }

        let lang_display = language.unwrap_or("all languages");
        debug!(
            "🔍 Search for '{}' in {} returned {} results",
            query_str,
            lang_display,
            results.len()
        );
        Ok(results)
    }

    // Keep existing methods for compatibility
    pub fn index_stored_page(&self, page: &StoredPage) -> Result<()> {
        let mut doc = TantivyDocument::new(); // FIXED: Use Document::new()
        doc.add_i64(self.id_field, page.id);
        doc.add_text(self.url_field, &page.url);
        doc.add_text(self.language_field, &page.language);
        doc.add_text(self.domain_field, &page.domain);
        doc.add_f64(self.quality_field, page.quality_score);

        // Index based on stored language
        match page.language.as_str() {
            "hi" => {
                if let Some(title) = &page.title {
                    doc.add_text(self.title_hi_field, title);
                }
                doc.add_text(self.content_hi_field, &page.content);
            }
            "kn" => {
                if let Some(title) = &page.title {
                    doc.add_text(self.title_kn_field, title);
                }
                doc.add_text(self.content_kn_field, &page.content);
            }
            "ta" => {
                if let Some(title) = &page.title {
                    doc.add_text(self.title_ta_field, title);
                }
                doc.add_text(self.content_ta_field, &page.content);
            }
            "te" => {
                if let Some(title) = &page.title {
                    doc.add_text(self.title_te_field, title);
                }
                doc.add_text(self.content_te_field, &page.content);
            }
            "ml" => {
                if let Some(title) = &page.title {
                    doc.add_text(self.title_ml_field, title);
                }
                doc.add_text(self.content_ml_field, &page.content);
            }
            "mr" => {
                if let Some(title) = &page.title {
                    doc.add_text(self.title_mr_field, title);
                }
                doc.add_text(self.content_mr_field, &page.content);
            }
            _ => {
                // English
                if let Some(title) = &page.title {
                    doc.add_text(self.title_en_field, title);
                }
                if let Some(description) = &page.description {
                    doc.add_text(self.description_en_field, description);
                }
                doc.add_text(self.content_en_field, &page.content);
            }
        }

        {
            let writer = self.writer.lock().unwrap();
            writer
                .add_document(doc)
                .map_err(|e| StorageError::SearchIndex(format!("Failed to add document: {}", e)))?;
        }
        debug!(
            "Indexed stored {} page: {} (ID: {})",
            page.language, page.url, page.id
        );
        Ok(())
    }

    pub fn batch_index_pages(&self, pages: &[(i64, PageData)]) -> Result<()> {
        let writer = self.writer.lock().unwrap();
        for (page_id, page) in pages {
            let detected_language = self.detect_content_language(&page.content);
            let mut doc = TantivyDocument::new(); // FIXED: Use Document::new()

            doc.add_i64(self.id_field, *page_id);
            doc.add_text(self.url_field, &page.url);
            doc.add_text(self.language_field, &detected_language);
            let domain = page.url.split('/').nth(2).unwrap_or("unknown");
            doc.add_text(self.domain_field, domain);
            doc.add_f64(self.quality_field, page.content_quality_score);

            // Index in appropriate language fields
            match detected_language.as_str() {
                "hi" => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_hi_field, title);
                    }
                    doc.add_text(self.content_hi_field, &page.content);
                }
                "kn" => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_kn_field, title);
                    }
                    doc.add_text(self.content_kn_field, &page.content);
                }
                "ta" => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_ta_field, title);
                    }
                    doc.add_text(self.content_ta_field, &page.content);
                }
                "te" => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_te_field, title);
                    }
                    doc.add_text(self.content_te_field, &page.content);
                }
                "ml" => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_ml_field, title);
                    }
                    doc.add_text(self.content_ml_field, &page.content);
                }
                "mr" => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_mr_field, title);
                    }
                    doc.add_text(self.content_mr_field, &page.content);
                }
                _ => {
                    if let Some(title) = &page.title {
                        doc.add_text(self.title_en_field, title);
                    }
                    if let Some(description) = &page.description {
                        doc.add_text(self.description_en_field, description);
                    }
                    doc.add_text(self.content_en_field, &page.content);
                }
            }

            writer
                .add_document(doc)
                .map_err(|e| StorageError::SearchIndex(format!("Failed to add document: {}", e)))?;
        }
        info!(
            "Batch indexed {} pages across multiple Indian languages",
            pages.len()
        );
        Ok(())
    }

    // Rest of existing methods remain the same
    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer
            .commit()
            .map_err(|e| StorageError::SearchIndex(format!("Failed to commit: {}", e)))?;
        Ok(())
    }

    pub fn delete_page(&self, page_id: i64) -> Result<()> {
        let term = Term::from_field_i64(self.id_field, page_id);
        {
            let writer = self.writer.lock().unwrap();
            writer.delete_term(term);
        }
        Ok(())
    }

    pub fn get_stats(&self) -> Result<SearchStats> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs() as u64;
        let index_size = self.calculate_index_size_bytes();

        Ok(SearchStats {
            total_documents: num_docs,
            index_size_bytes: index_size,
        })
    }

    pub fn optimize(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit().map_err(|e| {
            StorageError::SearchIndex(format!("Failed to commit during optimize: {}", e))
        })?;
        Ok(())
    }

    fn calculate_index_size_bytes(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone)]
pub struct SearchStats {
    pub total_documents: u64,
    pub index_size_bytes: u64,
}
