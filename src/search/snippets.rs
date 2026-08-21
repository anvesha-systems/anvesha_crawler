/// Generate search result snippets with context around matched terms
pub struct SnippetGenerator {
    max_length: usize,
    context_chars: usize,
}

impl SnippetGenerator {
    pub fn new() -> Self {
        Self {
            max_length: 200,   // Maximum snippet length in bytes
            context_chars: 80, // Characters before/after match
        }
    }

    pub fn with_max_length(mut self, length: usize) -> Self {
        self.max_length = length;
        self
    }

    /// Generate snippet from content with matched terms highlighted
    pub fn generate(&self, content: &str, query_terms: &[String], highlight: bool) -> String {
        if content.is_empty() {
            return String::from("No content available");
        }

        // Find first occurrence of any query term
        let content_lower = content.to_lowercase();
        let mut best_position = None;

        for term in query_terms {
            let term_lower = term.to_lowercase();
            if let Some(pos) = content_lower.find(&term_lower) {
                if best_position.is_none() || pos < best_position.unwrap() {
                    best_position = Some(pos);
                }
            }
        }

        // If no match found, return beginning of content
        let position = best_position.unwrap_or(0);

        // 🔥 FIX: Use char_indices() for Unicode-safe boundaries
        let chars: Vec<(usize, char)> = content.char_indices().collect();

        // Find char index for position
        let mut target_char_idx = 0;
        for (i, (byte_idx, _)) in chars.iter().enumerate() {
            if *byte_idx >= position {
                target_char_idx = i;
                break;
            }
        }

        // Calculate start and end in char indices
        let start_char = target_char_idx.saturating_sub(self.context_chars / 2);
        let end_char = (start_char + self.context_chars).min(chars.len());

        // Get byte positions from char indices
        let start_byte = if start_char > 0 {
            chars[start_char].0
        } else {
            0
        };

        let end_byte = if end_char < chars.len() {
            chars[end_char].0
        } else {
            content.len()
        };

        // Extract snippet (now safe!)
        let mut snippet = content[start_byte..end_byte].to_string();

        // Trim to word boundaries
        if start_byte > 0 {
            // Find first space after start
            if let Some(space_pos) = snippet.find(|c: char| c.is_whitespace()) {
                snippet = snippet[space_pos..].trim_start().to_string();
                snippet.insert_str(0, "...");
            }
        }

        if end_byte < content.len() {
            // Find last space before end
            if let Some(space_pos) = snippet.rfind(|c: char| c.is_whitespace()) {
                snippet.truncate(space_pos);
                snippet.push_str("...");
            }
        }

        // Apply highlighting if requested
        if highlight {
            snippet = self.highlight_terms(&snippet, query_terms);
        }

        snippet
    }

    /// Highlight matched terms in snippet (Unicode-safe)
    fn highlight_terms(&self, text: &str, query_terms: &[String]) -> String {
        let mut result = text.to_string();

        for term in query_terms {
            let term_lower = term.to_lowercase();
            let result_lower = result.to_lowercase();

            // Find all matches
            let matches: Vec<(usize, usize)> = result_lower
                .match_indices(&term_lower)
                .map(|(start, matched)| (start, start + matched.len()))
                .collect();

            // Apply highlighting in reverse order to maintain positions
            for (start, end) in matches.iter().rev() {
                // 🔥 SAFE: match_indices returns valid UTF-8 boundaries
                let before = &result[..*start];
                let matched = &result[*start..*end];
                let after = &result[*end..];

                result = format!("{}**{}**{}", before, matched, after);
            }
        }

        result
    }

    /// Extract query terms from query string
    pub fn extract_terms(query: &str) -> Vec<String> {
        query
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }
}

impl Default for SnippetGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_terms() {
        let terms = SnippetGenerator::extract_terms("web crawler search");
        assert_eq!(terms, vec!["web", "crawler", "search"]);
    }

    #[test]
    fn test_snippet_generation() {
        let generator = SnippetGenerator::new();
        let content =
            "A web crawler is an Internet bot that systematically browses the World Wide Web.";
        let terms = vec!["crawler".to_string()];

        let snippet = generator.generate(content, &terms, false);
        assert!(snippet.contains("crawler"));
    }

    #[test]
    fn test_highlighting() {
        let generator = SnippetGenerator::new();
        let content = "A web crawler is an Internet bot.";
        let terms = vec!["web".to_string(), "crawler".to_string()];

        let snippet = generator.generate(content, &terms, true);
        assert!(snippet.contains("**web**"));
        assert!(snippet.contains("**crawler**"));
    }

    #[test]
    fn test_unicode_content() {
        let generator = SnippetGenerator::new();
        let content = "爬網蟲械（Web crawler），亦稱網蛛（Spider）";
        let terms = vec!["crawler".to_string()];

        let snippet = generator.generate(content, &terms, false);
        assert!(snippet.contains("crawler"));
    }
}
