use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerConfig {
    pub crawler: CrawlerSettings,
    pub network: NetworkSettings,
    pub storage: StorageSettings,
    pub algorithms: AlgorithmSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerSettings {
    pub max_depth: u32,
    pub max_pages: usize,
    pub concurrent_requests: usize,
    pub seed_urls: Vec<String>,
    pub user_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub request_timeout_secs: u64,
    pub request_delay_ms: u64,
    pub max_retries: u32,
    pub respect_robots_txt: bool,

    pub max_content_size_mb: usize,
    pub user_agents: Vec<String>,
    pub max_redirects: u32,
    pub connect_timeout_secs: u64,
}

fn default_max_connections() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub database_url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    pub redis_url: Option<String>,
    pub enable_caching: bool,
    pub storage_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmSettings {
    pub primary_algorithm: String, // "bfs", "best_first", "shark_search"
    pub enable_opic: bool,
    pub priority_boost_domains: Vec<String>,
}

impl CrawlerConfig {
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: CrawlerConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default() -> Self {
        Self {
            crawler: CrawlerSettings {
                max_depth: 3,
                max_pages: 10000,
                concurrent_requests: 10,
                seed_urls: vec![],
                user_agent: "SearchEngineBot/1.0".to_string(),
            },
            network: NetworkSettings {
                request_timeout_secs: 30,
                request_delay_ms: 1000,
                max_retries: 3,
                respect_robots_txt: true,
                max_content_size_mb: 10,
                user_agents: vec![
                    "Mozilla/5.0 (compatible; WebCrawler/1.0; +http://example.com/bot)".to_string(),
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string(),
                ],
                max_redirects: 10,
                connect_timeout_secs: 10
            },
            storage: StorageSettings {
                database_url: "postgresql://localhost/crawler".to_string(),
                max_connections: 10,
                redis_url: None,
                enable_caching: true,
                storage_path: "./data".to_string(),
            },
            algorithms: AlgorithmSettings {
                primary_algorithm: "bfs".to_string(),
                enable_opic: true,
                priority_boost_domains: vec![
                    "wikipedia.org".to_string(),
                    ".edu".to_string(),
                    ".gov".to_string(),
                ],
            },
        }
    }
}
