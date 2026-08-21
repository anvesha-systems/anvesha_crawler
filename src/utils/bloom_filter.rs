use std::collections::HashSet;

// simple placeholder you can implement a real bloom filter later
pub struct BloomFilter {
    seen: HashSet<String>,
}

impl BloomFilter {
    pub fn new(_capacity: usize) -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    pub fn contains(&self, item: &str) -> bool {
        self.seen.contains(item)
    }

    pub fn insert(&mut self, item: String) {
        self.seen.insert(item);
    }
}
