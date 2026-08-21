use crate::models::PageData;
use crate::storage::repository::PageRepository;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{Index, IndexWriter, doc};
use tracing::{info, warn};

use super::schema::SearchSchema;

pub struct SearchIndexer {
    index: Index,
    search_schema: SearchSchema,
}

impl SearchIndexer {
    pub fn new(index_path: &Path) -> tantivy::Result<Self> {
        let index = SearchSchema::open_or_create(index_path)?;
        let search_schema = SearchSchema::build();

        Ok(Self {
            index,
            search_schema,
        })
    }

    pub fn index_page(&self, page: &PageData) -> tantivy::Result<()> {
        let mut index_writer = self.index.writer(50_000_000)?;

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(self.search_schema.url_field, &page.url);

        if let Some(ref title) = page.title {
            doc.add_text(self.search_schema.title_field, &title);
        }

        doc.add_text(self.search_schema.content_field, &page.content);
        doc.add_text(
            self.search_schema.domain_field,
            &self.extract_domain(&page.url),
        );
        doc.add_f64(self.search_schema.quality_field, page.content_quality_score);

        index_writer.add_document(doc)?;
        index_writer.commit()?;

        info!("Indexed page: {}", page.url);
        Ok(())
    }

    pub async fn index_all_pages(&self, repository: &PageRepository) -> tantivy::Result<()> {
        info!("Starting full indexing of all pages...");

        let mut index_writer = self.index.writer(50_000_000)?;
        let mut count = 0;

        // get all pages from database
        let filter = crate::storage::models::PageFilter::new().with_limit(10000);
        let pages = repository
            .get_pages(&filter)
            .await
            .map_err(|e| tantivy::TantivyError::InternalError(e.to_string()))?;

        for stored_pages in pages {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(self.search_schema.url_field, &stored_pages.url);

            if let Some(ref title) = stored_pages.title {
                doc.add_text(self.search_schema.title_field, &title);
            }
            doc.add_text(self.search_schema.content_field, &stored_pages.content);
            doc.add_text(self.search_schema.domain_field, &stored_pages.domain);
            doc.add_f64(self.search_schema.quality_field, stored_pages.quality_score);
            doc.add_f64(
                self.search_schema.pagerank_field,
                stored_pages.pagerank.unwrap_or(0.0),
            );
            doc.add_f64(
                self.search_schema.tfidf_field,
                stored_pages.tfidf_score.unwrap_or(0.0),
            );
            index_writer.add_document(doc)?;
            count += 1;
        }
        index_writer.commit()?;
        info!("Indexed {} pages successfully", count);

        Ok(())
    }

    fn extract_domain(&self, url: &str) -> String {
        url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_else(|| "unknown".to_string())
    }
}
