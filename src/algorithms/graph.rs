use crate::storage::Result;
use crate::storage::models::PageFilter;
use clap::builder::Str;
use log::info;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct LinkGraph {
    // URL -> list of urls it links to
    pub outbounds: HashMap<String, Vec<String>>,

    // URL -> List of URLs linking to it
    pub inbounds: HashMap<String, Vec<String>>,

    // All unique urls in graph
    pub nodes: Vec<String>,
}

impl LinkGraph {
    pub fn new() -> Self {
        Self {
            outbounds: HashMap::new(),
            inbounds: HashMap::new(),
            nodes: Vec::new(),
        }
    }
    pub async fn from_database(db: &crate::storage::repository::PageRepository) -> Result<Self> {
        use tracing::info;

        info!("Building link graph from database");

        // get all pages

        let filter = PageFilter::new();
        let pages = db.get_pages(&filter).await?;

        info!("Loaded {} pages from database", pages.len());

        let mut nodes = Vec::new();
        let mut outbounds: HashMap<String, Vec<String>> = HashMap::new();
        let mut inbounds: HashMap<String, Vec<String>> = HashMap::new();

        // collect all unique urls
        let all_urls: HashSet<String> = pages.iter().map(|p| p.url.clone()).collect();
        nodes.extend(all_urls.iter().cloned());

        // get all links
        let links = db.get_all_links().await?;

        for (source_url, target_url) in links {
            let source = source_url;
            let target = target_url;

            //     add to outbound
            outbounds
                .entry(source.clone())
                .or_insert_with(Vec::new)
                .push(target.clone());

            // add to inbounds

            inbounds
                .entry(target.clone())
                .or_insert_with(Vec::new)
                .push(source.clone());
        }

        // ensure all nodes have entries (even if no links)
        for url in &nodes {
            outbounds.entry(url.clone()).or_insert_with(Vec::new);
            inbounds.entry(url.clone()).or_insert_with(Vec::new);
        }

        let edge_count: usize = outbounds.values().map(|v| v.len()).sum();

        info!(
            "Link graph built: {} nodes, {} edges",
            nodes.len(),
            edge_count
        );

        Ok(Self {
            outbounds,
            inbounds,
            nodes,
        })
    }

    /// Build link graph from PageRepository
    pub async fn from_repository(
        repo: &crate::storage::repository::PageRepository,
    ) -> crate::storage::Result<Self> {
        use crate::algorithms::graph::PageFilter;
        use tracing::info;

        info!("Building link graph from database...");

        // Get all pages using existing get_pages method
        let filter = PageFilter::new();
        let pages = repo.get_pages(&filter).await?;

        info!("Loaded {} pages from database", pages.len());

        let mut nodes = Vec::new();
        let mut outbound: HashMap<String, Vec<String>> = HashMap::new();
        let mut inbound: HashMap<String, Vec<String>> = HashMap::new();

        // Collect all unique URLs
        let all_urls: HashSet<String> = pages.iter().map(|p| p.url.clone()).collect();
        nodes.extend(all_urls.iter().cloned());

        // Get all links from database using the new method
        let links = repo.get_all_links().await?;

        info!("Loaded {} links from database", links.len());

        // Build outbound and inbound maps
        for (source_url, target_url) in links {
            // Add to outbound
            outbound
                .entry(source_url.clone())
                .or_insert_with(Vec::new)
                .push(target_url.clone());

            // Add to inbound
            inbound
                .entry(target_url.clone())
                .or_insert_with(Vec::new)
                .push(source_url.clone());
        }

        // Ensure all nodes have entries (even if no links)
        for url in &nodes {
            outbound.entry(url.clone()).or_insert_with(Vec::new);
            inbound.entry(url.clone()).or_insert_with(Vec::new);
        }

        let edge_count: usize = outbound.values().map(|v| v.len()).sum();

        info!(
            "Link graph built: {} nodes, {} edges",
            nodes.len(),
            edge_count
        );

        Ok(Self {
            outbounds: outbound,
            inbounds: inbound,
            nodes,
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn outbound_count(&self, url: &str) -> usize {
        self.outbounds.get(url).map(|v| v.len()).unwrap_or(0)
    }

    pub fn inbound_count(&self, url: &str) -> usize {
        self.inbounds.get(url).map(|v| v.len()).unwrap_or(0)
    }

    pub fn dangling_nodes(&self) -> Vec<&String> {
        self.nodes
            .iter()
            .filter(|url| self.outbound_count(url) == 0)
            .collect()
    }
}
