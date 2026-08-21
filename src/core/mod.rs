//! core crawler components

pub mod crawler;
pub mod page_processor;
pub mod scheduler;
mod tests;
pub mod url_frontier;

pub use page_processor::PageProcessor;
pub use scheduler::CrawlScheduler;
pub use url_frontier::UrlFrontier;
