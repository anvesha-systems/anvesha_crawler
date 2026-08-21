use std::path::Path;
use tantivy::Index;
use tantivy::schema::*;

#[derive(Debug)]
pub struct SearchSchema {
    pub schema: Schema,
    pub url_field: Field,
    pub title_field: Field,
    pub content_field: Field,
    pub domain_field: Field,
    pub quality_field: Field,
    pub pagerank_field: Field,
    pub tfidf_field: Field,
}

impl SearchSchema {
    pub fn build() -> Self {
        let mut schema_builder = Schema::builder();

        // url field - stored index
        let url_field = schema_builder.add_text_field("url", TEXT | STORED);

        // title field searchable with high boost
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);

        // content field - searchable
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);

        // Domain field - faceted search
        let domain_field = schema_builder.add_text_field("domain", TEXT | STORED);

        // quality score for ranking
        let quality_field = schema_builder.add_text_field("quality", TEXT | STORED);

        let pagerank_field = schema_builder.add_f64_field("pagerank", FAST | STORED);

        let tfidf_field = schema_builder.add_f64_field("tfidf", FAST | STORED);

        let schema = schema_builder.build();

        Self {
            schema,
            url_field,
            title_field,
            content_field,
            domain_field,
            quality_field,
            pagerank_field,
            tfidf_field,
        }
    }

    pub fn create_index(index_path: &Path) -> tantivy::Result<Index> {
        let search_schema = Self::build();

        if !index_path.exists() {
            std::fs::create_dir_all(index_path)?;
        }

        Index::create_in_dir(index_path, search_schema.schema)
    }

    pub fn open_or_create(index_path: &Path) -> tantivy::Result<Index> {
        if index_path.exists() && index_path.read_dir()?.next().is_some() {
            Index::open_in_dir(index_path)
        } else {
            Self::create_index(index_path)
        }
    }
}
