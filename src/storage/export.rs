// src/storage/export.rs
use crate::storage::Result;
use crate::storage::models::PageFilter;
use crate::storage::repository::PageRepository;
use csv::WriterBuilder;
use serde::Serialize;
use std::fs::File;
use std::path::Path;

#[derive(Serialize)]
struct PageCsv {
    id: i64,
    url: String,
    domain: String,
    title: String,
    quality_score: f64,
    word_count: i32,
    crawled_at: String,
}

pub struct DataExporter<'a> {
    repo: &'a PageRepository,
}

impl<'a> DataExporter<'a> {
    pub fn new(repo: &'a PageRepository) -> Self {
        Self { repo }
    }

    pub async fn pages_to_json<P: AsRef<Path>>(&self, filter: &PageFilter, path: P) -> Result<()> {
        let pages = self.repo.get_pages(filter).await?;
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &pages)?;
        Ok(())
    }

    pub async fn pages_to_csv<P: AsRef<Path>>(&self, filter: &PageFilter, path: P) -> Result<()> {
        let pages = self.repo.get_pages(filter).await?;
        let file = File::create(path)?;
        let mut wtr = WriterBuilder::new().from_writer(file);
        for p in pages {
            let row = PageCsv {
                id: p.id,
                url: p.url,
                domain: p.domain,
                title: p.title.unwrap_or_default(),
                quality_score: p.quality_score,
                word_count: p.word_count,
                crawled_at: p.crawled_at.to_rfc3339(),
            };
            wtr.serialize(row)
                .map_err(|e| crate::storage::StorageError::Export(e.to_string()))?;
        }
        wtr.flush()?;
        Ok(())
    }
}
