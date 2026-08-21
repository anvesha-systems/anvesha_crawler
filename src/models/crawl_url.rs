use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrawlUrl {
    pub url: String,
    pub priority: f64,
    pub depth: u32,
    pub discovered_at: u64, // Unix timestamp
}

// Implement ordering - HIGHER priority should be GREATER (for max-heap behavior)
impl Ord for CrawlUrl {
    fn cmp(&self, other: &Self) -> Ordering {
        // ✅ FIXED: Higher priority should be Greater
        match self.priority.partial_cmp(&other.priority) {
            Some(Ordering::Equal) => {
                // If priorities are equal, prefer lower depth (shallow pages first)
                match other.depth.cmp(&self.depth) {
                    // Reverse depth comparison
                    Ordering::Equal => {
                        // If depth is equal, prefer earlier discovered (FIFO)
                        other.discovered_at.cmp(&self.discovered_at) // Reverse time comparison
                    }
                    depth_ordering => depth_ordering,
                }
            }
            Some(priority_ordering) => priority_ordering,
            None => Ordering::Equal, // Handle NaN case
        }
    }
}

impl PartialOrd for CrawlUrl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CrawlUrl {}
