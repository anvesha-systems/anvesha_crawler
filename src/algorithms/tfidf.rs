use std::collections::{HashMap, HashSet};
use tantivy::schema::Value;
use tracing::info;

lazy_static::lazy_static! {
    static ref STOP_WORDS: HashSet<&'static str> = {
        [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to",
            "for", "of", "as", "by", "is", "was", "are", "were", "been",
            "be", "have", "has", "had", "do", "does", "did", "will", "would",
            "should", "could", "may", "might", "can", "this", "that", "these",
            "those", "it", "its", "with", "from", "into", "out", "over", "under",
            "about", "which", "who", "where", "when", "why", "how", "all", "each",
            "any", "some", "more", "than", "not", "only", "such", "here", "there",
            "hlist", "archived", "using", "also", "their", "them", "his", "her"
        ].iter().copied().collect()
    };
}

/// TF-IDF Calculator for document corpus
pub struct TfIdfCalculator {
    /// term -> (doc_id -> count)
    term_doc_freq: HashMap<String, HashMap<String, usize>>,

    /// term -> number of documents containing it
    document_freq: HashMap<String, usize>,

    /// doc_id -> total terms in document
    doc_lengths: HashMap<String, usize>,

    /// Total number of documents in corpus
    total_docs: usize,
}

impl TfIdfCalculator {
    pub fn new() -> Self {
        Self {
            term_doc_freq: HashMap::new(),
            document_freq: HashMap::new(),
            doc_lengths: HashMap::new(),
            total_docs: 0,
        }
    }

    /// Build TF-IDF index from corpus
    ///
    /// # Arguments
    /// * `documents` - Vec of (doc_id, content) tuples
    pub fn build_from_corpus(&mut self, documents: &[(String, String)]) {
        info!(
            "Building TF-IDF index from {} documents...",
            documents.len()
        );

        self.total_docs = documents.len();

        for (doc_id, content) in documents {
            let terms = Self::tokenize(content);
            let term_counts = Self::count_terms(&terms);

            self.doc_lengths.insert(doc_id.clone(), terms.len());

            // Update term frequencies
            for (term, count) in term_counts {
                // Update document frequency
                *self.document_freq.entry(term.clone()).or_insert(0) += 1;

                // Update term-document frequency
                self.term_doc_freq
                    .entry(term)
                    .or_insert_with(HashMap::new)
                    .insert(doc_id.clone(), count);
            }
        }

        info!(
            "TF-IDF index built: {} unique terms",
            self.term_doc_freq.len()
        );
    }

    /// Calculate Term Frequency for a term in a document
    ///
    /// TF = (term count in doc) / (total terms in doc)
    pub fn calculate_tf(&self, term: &str, doc_id: &str) -> f64 {
        if let Some(doc_terms) = self.term_doc_freq.get(term) {
            if let Some(&count) = doc_terms.get(doc_id) {
                let doc_length = self.doc_lengths.get(doc_id).unwrap_or(&1);
                return count as f64 / *doc_length as f64;
            }
        }
        0.0
    }

    /// Calculate Inverse Document Frequency for a term
    ///
    /// IDF = log(total_docs / docs_containing_term)
    pub fn calculate_idf(&self, term: &str) -> f64 {
        if let Some(&df) = self.document_freq.get(term) {
            if df > 0 {
                return ((self.total_docs as f64) / (df as f64)).ln();
            }
        }
        0.0
    }

    /// Calculate TF-IDF score for a term in a document
    ///
    /// TF-IDF = TF × IDF
    pub fn calculate_tfidf(&self, term: &str, doc_id: &str) -> f64 {
        let tf = self.calculate_tf(term, doc_id);
        let idf = self.calculate_idf(term);
        tf * idf
    }

    /// Get top N terms for a document by TF-IDF score
    pub fn get_top_terms(&self, doc_id: &str, n: usize) -> Vec<(String, f64)> {
        let mut scores: Vec<(String, f64)> = self
            .term_doc_freq
            .keys()
            .map(|term| {
                let score = self.calculate_tfidf(term, doc_id);
                (term.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(n).collect()
    }

    /// Calculate cosine similarity between query and document using TF-IDF
    pub fn query_document_similarity(&self, query: &str, doc_id: &str) -> f64 {
        let query_terms = Self::tokenize(query);
        let query_term_counts = Self::count_terms(&query_terms);

        let mut dot_product = 0.0;
        let mut query_magnitude = 0.0;
        let mut doc_magnitude = 0.0;

        // Calculate dot product and magnitudes
        for term in &query_terms {
            let query_tf =
                *query_term_counts.get(term).unwrap_or(&0) as f64 / query_terms.len() as f64;
            let doc_tfidf = self.calculate_tfidf(&term, doc_id);
            let query_idf = self.calculate_idf(&term);
            let query_tfidf = query_tf * query_idf;

            dot_product += query_tfidf * doc_tfidf;
            query_magnitude += query_tfidf * query_tfidf;
            doc_magnitude += doc_tfidf * doc_tfidf;
        }

        // Cosine similarity
        if query_magnitude > 0.0 && doc_magnitude > 0.0 {
            dot_product / (query_magnitude.sqrt() * doc_magnitude.sqrt())
        } else {
            0.0
        }
    }

    /// Tokenize text into terms (simple whitespace + lowercase)
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|s| s.len() > 2) // Filter short words
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .filter(|s| !STOP_WORDS.contains(s))
            .map(String::from)
            .collect()
    }

    /// Count term frequencies in token list
    fn count_terms(terms: &[String]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for term in terms {
            *counts.entry(term.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Get statistics about the TF-IDF index
    pub fn get_stats(&self) -> TfIdfStats {
        TfIdfStats {
            total_documents: self.total_docs,
            unique_terms: self.term_doc_freq.len(),
            avg_doc_length: if self.doc_lengths.is_empty() {
                0.0
            } else {
                self.doc_lengths.values().sum::<usize>() as f64 / self.doc_lengths.len() as f64
            },
        }
    }
}

/// Statistics about TF-IDF index
#[derive(Debug, Clone)]
pub struct TfIdfStats {
    pub total_documents: usize,
    pub unique_terms: usize,
    pub avg_doc_length: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfidf_basic() {
        let mut calculator = TfIdfCalculator::new();

        let docs = vec![
            ("doc1".to_string(), "web crawler crawls the web".to_string()),
            (
                "doc2".to_string(),
                "web design for modern websites".to_string(),
            ),
            (
                "doc3".to_string(),
                "search engine crawler technology".to_string(),
            ),
        ];

        calculator.build_from_corpus(&docs);

        // "crawler" appears in 2/3 docs
        let idf_crawler = calculator.calculate_idf("crawler");
        assert!(idf_crawler > 0.0);

        // "web" appears in 2/3 docs
        let idf_web = calculator.calculate_idf("web");
        assert!(idf_web > 0.0);

        // Calculate TF-IDF for "crawler" in doc1
        let tfidf = calculator.calculate_tfidf("crawler", "doc1");
        assert!(tfidf > 0.0);

        println!("Basic TF-IDF test passed");
    }

    #[test]
    fn test_top_terms() {
        let mut calculator = TfIdfCalculator::new();

        let docs = vec![(
            "doc1".to_string(),
            "rust programming language is great for systems".to_string(),
        )];

        calculator.build_from_corpus(&docs);

        let top_terms = calculator.get_top_terms("doc1", 3);
        assert!(top_terms.len() <= 3);

        println!("Top terms test passed");
    }
}
