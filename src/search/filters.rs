use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilter {
    pub domain: Option<String>,
    pub min_quality: Option<f64>,
    pub max_quality: Option<f64>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
}

impl SearchFilter {
    pub fn new() -> Self {
        Self {
            domain: None,
            max_quality: None,
            min_quality: None,
            after: None,
            before: None,
        }
    }
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }

    pub fn with_max_quality(mut self, quality: f64) -> Self {
        self.max_quality = Some(quality);
        self
    }

    pub fn with_min_quality(mut self, quality: f64) -> Self {
        self.min_quality = Some(quality);
        self
    }

    pub fn with_after(mut self, date: DateTime<Utc>) -> Self {
        self.after = Some(date);
        self
    }

    pub fn with_before(mut self, date: DateTime<Utc>) -> Self {
        self.before = Some(date);
        self
    }

    pub fn has_filters(&self) -> bool {
        self.domain.is_some()
            || self.min_quality.is_some()
            || self.max_quality.is_some()
            || self.after.is_some()
            || self.min_quality.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SortBy {
    Relevance,
    Quality,
    PageRank,
    TfIdf,
    Date,
}

impl Default for SortBy {
    fn default() -> Self {
        SortBy::Relevance
    }
}

impl std::str::FromStr for SortBy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "relevance" => Ok(SortBy::Relevance),
            "quality" => Ok(SortBy::Quality),
            "pagerank" | "rank" => Ok(SortBy::PageRank),
            "tfidf" | "idf" => Ok(SortBy::TfIdf),
            "date" => Ok(SortBy::Date),
            _ => Err(format!("Invalid sort option: {}", s)),
        }
    }
}
