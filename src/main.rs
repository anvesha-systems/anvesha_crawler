use clap::Parser;
use crawler::algorithms::{LinkGraph, PageRankCalculator};
use crawler::storage::database::{Database, DatabaseConfig};
use crawler::storage::models::PageFilter;
use crawler::storage::repository::PageRepository;
use crawler::{CrawlerConfig, WebCrawler, init};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "search-crawler")]
#[command(about = "A modular web crawler for search engines")]
struct Args {
    #[arg(short, long, default_value = "config/default.toml")]
    config: String,

    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
enum Commands {
    Crawl {
        #[arg(short, long)]
        seed_urls: Vec<String>,
        #[arg(long)]
        save_to_db: bool,
        #[arg(long, default_value = "10")]
        max_pages: usize,
    },
    Index {
        #[arg(long, default_value = "./search_index")]
        index_path: String,
    },
    Search {
        /// Search query string
        query: String,

        #[arg(long, default_value = "./search_index")]
        index_path: String,

        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,

        /// FIX: Add offset parameter
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Filter by domain
        #[arg(long)]
        domain: Option<String>,

        /// Minimum quality score
        #[arg(long)]
        min_quality: Option<f64>,

        /// Maximum quality score
        #[arg(long)]
        max_quality: Option<f64>,

        /// Sort by: relevance, quality, or date
        #[arg(long, default_value = "relevance")]
        sort: String,

        /// Generate content snippets
        #[arg(long)]
        snippets: bool,

        /// Highlight matched terms
        #[arg(long)]
        highlight: bool,
    },
    CalculatePageRank {
        #[arg(long, default_value = "10")]
        top: usize,
    },

    CalculateTfIdf {
        // show up n terms for each document
        #[arg(long, default_value = "10")]
        top: Option<usize>,
    },

    Api {
        #[arg(short, long, default_value = "3000")]
        port: u16,

        #[arg(long, default_value = "./search_index")]
        index_path: String,
    },
    Stats,
}

#[tokio::main]
async fn main() -> crawler::Result<()> {
    let args = Args::parse();

    // Initialize logging and metrics
    init().await?;

    // Load configuration
    let config = CrawlerConfig::from_file(&args.config)?;
    info!("Loaded configuration from: {}", args.config);

    match args.command {
        Some(Commands::Crawl {
            seed_urls,
            save_to_db,
            max_pages,
        }) => {
            let mut crawler_config = config;

            // Update seed URLs if provided
            if !seed_urls.is_empty() {
                crawler_config.crawler.seed_urls = seed_urls;
            }

            // Update max pages if provided
            crawler_config.crawler.max_pages = max_pages;

            // SIMPLE: Initialize database if save_to_db is true
            let repository = if save_to_db {
                info!("Database storage enabled - initializing PostgreSQL database");

                let db_config = DatabaseConfig {
                    database_url: crawler_config.storage.database_url.clone(),
                    max_connections: crawler_config.storage.max_connections,
                    enable_wal_mode: false,
                    enable_foreign_keys: true,
                };

                // Connect and migrate database
                let pool = Database::connect(&db_config).await?;
                Database::migrate(&pool).await?;
                info!("Database initialized and migrations completed");

                Some(PageRepository::new(pool))
            } else {
                info!("Running crawler without database storage");
                None
            };

            // 🔥 SIMPLE: Just create crawler normally
            let crawler = WebCrawler::new(crawler_config).await?;

            // SIMPLE: Pass repository to the crawl method
            crawler.start_crawling_with_repository(repository).await?;
        }

        Some(Commands::Index { index_path }) => {
            use crawler::search::SearchIndexer;
            use crawler::storage::database::{Database, DatabaseConfig};
            use crawler::storage::repository::PageRepository;

            info!("Starting search indexing...");

            // connect to database
            let db_config = DatabaseConfig::default();
            let pool = Database::connect(&db_config).await?;
            let repository = PageRepository::new(pool);

            // create indexer and index all pages
            let indexer = SearchIndexer::new(Path::new(&index_path))?;
            indexer.index_all_pages(&repository).await?;

            println!("Indexing completed!");
        }

        Some(Commands::CalculatePageRank { top }) => {
            info!("Starting PageRank calculation...");

            // Initialize database connection
            let db_config = DatabaseConfig::default();
            let pool = Database::connect(&db_config).await?;
            let repository = PageRepository::new(pool);

            // Load graph from repository
            let graph = LinkGraph::from_repository(&repository).await?;
            info!(
                "Graph Stats: {} nodes, {} dangling nodes",
                graph.node_count(),
                graph.dangling_nodes().len()
            );

            // Calculate PageRank
            let calculator = PageRankCalculator::new();
            let ranks = calculator.calculate(&graph);

            // Store PageRank values using batch update for efficiency
            info!("Storing PageRank values...");
            let ranks_vec: Vec<(String, f64)> = ranks
                .iter()
                .map(|(url, rank)| (url.clone(), *rank))
                .collect();

            repository.batch_update_pagerank(&ranks_vec).await?;

            // Display top pages
            let top_pages = calculator.get_top_pages(&ranks, top);
            println!("\nTop {} Pages by PageRank:\n", top);
            println!("{:<6} {:<12} URL", "Rank", "PageRank");
            println!("{}", "=".repeat(80));

            for (i, (url, rank)) in top_pages.iter().enumerate() {
                println!("{:<6} {:<12.6} {}", format!("{}.", i + 1), rank, url);
            }

            println!("\nPageRank calculation complete!");
        }

        Some(Commands::CalculateTfIdf { top }) => {
            use crawler::algorithms::TfIdfCalculator;
            use crawler::storage::{database::Database, repository::PageRepository};

            let db_config = DatabaseConfig::default();
            let pool = Database::connect(&db_config).await?;
            let repository = PageRepository::new(pool);

            let pages = repository.get_pages(&PageFilter::new()).await?;
            println!("📊 Loaded {} documents", pages.len());

            // Build corpus: (doc_id=url_hash preferred for stability, content)
            let corpus: Vec<(String, String)> = pages
                .iter()
                .map(|p| (p.url_hash.clone(), p.content.clone()))
                .collect();

            let mut tfidf = TfIdfCalculator::new();
            tfidf.build_from_corpus(&corpus);

            // For each doc, compute a single “magnitude” score to store
            // Magnitude = sqrt(sum over terms of (tfidf(term, doc))^2)
            for p in &pages {
                let doc_id = &p.url_hash;
                let top_terms = tfidf.get_top_terms(doc_id, top.unwrap_or(256)); // sample top N
                let magnitude = top_terms.iter().map(|(_, s)| s * s).sum::<f64>().sqrt();

                repository.update_tfidf_score(doc_id, magnitude).await?;
                println!("{} tfidf_score={:.6}", p.url, magnitude);
            }

            println!("TF-IDF scores updated");
        }

        Some(Commands::Search {
            query,
            index_path,
            limit,
            domain,
            offset,
            min_quality,
            max_quality,
            sort,
            snippets,
            highlight,
        }) => {
            use crawler::search::SearchQuery;
            use crawler::search::filters::{SearchFilter, SortBy};
            use std::path::Path;
            use std::str::FromStr;

            info!("Searching for : '{}'", query);

            // Build filters
            let mut filters = SearchFilter::new();
            if let Some(domain) = domain {
                filters = filters.with_domain(domain.clone());
                info!("Filter : Domain = '{}'", domain);
            }

            if let Some(min_q) = min_quality {
                filters = filters.with_min_quality(min_q);
                info!("   Filter: min_quality = {}", min_q);
            }
            if let Some(max_q) = max_quality {
                filters = filters.with_max_quality(max_q);
                info!("   Filter: max_quality = {}", max_q);
            }

            // Parse sort option
            let sort_by = SortBy::from_str(&sort).unwrap_or_else(|_e| {
                warn!("Invalid sort option '{}', using relevance", sort);
                SortBy::Relevance
            });

            // create search query engine
            let search_engine = SearchQuery::new(Path::new(&index_path))?;

            // execute search
            let results = search_engine.search_with_filters(
                &query, limit, filters, sort_by, offset, snippets, highlight,
            )?;

            // display results
            println!("\n Search results for : '{}'\n", query);
            println!("Found {} results : \n", results.len());

            for (i, result) in results.iter().enumerate() {
                println!(
                    " {}. {} (score : {:.3}, pagerank:  {:.6}, tfidf: {:.6})",
                    i + 1,
                    result.url,
                    result.score,
                    result.pagerank,
                    result.tfidf
                );
                if let Some(ref title) = result.title {
                    println!("Title: {}", title);
                }
                println!(
                    " Domain: {} | Quality: {:.3}",
                    result.domain, result.quality_score
                );

                // printing the snippet
                if snippets {
                    match &result.snippet {
                        Some(snippet) => {
                            println!("Snippet: {}", snippet);
                        }
                        None => {
                            println!("Snippet: Snippet requested but not generated");
                        }
                    }
                }
                println!();
            }
        }

        Some(Commands::Api { port, index_path }) => {
            use crawler::SearchEngine;
            use crawler::api::{AppState, build_router};

            let index = Path::new(&index_path);
            if !index.exists() {
                eprintln!(
                    "Error: index path '{}' does not exist. Run `crawler index` first.",
                    index_path
                );
                std::process::exit(1);
            }

            let engine = SearchEngine::new(index)?;
            let api_key = std::env::var("ANVESHA_SEARCH_API_KEY").ok();
            let auth_enabled = api_key.is_some();
            let state = Arc::new(AppState { engine, api_key });

            let router = build_router(state);
            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;

            info!(port = port, index_path = %index_path, auth_enabled = auth_enabled, "API server starting");
            axum::serve(listener, router).await?;
        }
        Some(Commands::Stats) => {
            println!("Crawler Statistics:");
        }
        None => {
            let crawler = WebCrawler::new(config).await?;
            crawler.start_crawling().await?;
        }
    }

    Ok(())
}
