// Database connection and management

use crate::storage::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use tracing::{info, warn};

pub type DatabasePool = Pool<Postgres>;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub enable_wal_mode: bool,
    pub enable_foreign_keys: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://crawler_user:crawler_pass@localhost:5432/crawler_db"
                .to_string(),
            max_connections: 10,
            enable_wal_mode: true,
            enable_foreign_keys: true,
        }
    }
}

pub struct Database;

impl Database {
    // create a new db connection pool
    pub async fn connect(config: &DatabaseConfig) -> Result<DatabasePool> {
        info!("Connecting to database : {}", config.database_url);

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url)
            .await?;

        info!("Database connected successfully");
        Ok(pool)
    }

    // Run database migrations using sqlx's built-in migration runner.
    // Migrations are applied in version order from the ./migrations directory.
    // The _sqlx_migrations table tracks which migrations have been applied,
    // making this safe to call on an existing database.
    pub async fn migrate(pool: &DatabasePool) -> Result<()> {
        info!("Running migrations...");
        sqlx::migrate!("./migrations").run(pool).await?;
        info!("Database migrations complete");
        Ok(())
    }

    // check db health
    pub async fn health_check(pool: &DatabasePool) -> bool {
        match sqlx::query("SELECT 1 as health_check")
            .fetch_one(pool)
            .await
        {
            Ok(row) => {
                let result: i32 = row.get("health_check");
                result == 1
            }
            Err(e) => {
                warn!("Database health check failed, {}", e);
                false
            }
        }
    }

    // Get database statistics
    pub async fn get_database_stats(pool: &DatabasePool) -> Result<crate::storage::DatabaseStats> {
        let row = sqlx::query(
            r#"
                SELECT
                    (SELECT COUNT(*) FROM pages) as total_pages,
                    (SELECT COUNT(*) FROM links) as total_links,
                    (SELECT COUNT(DISTINCT domain) FROM pages) as total_domains,
                    (SELECT AVG(quality_score) FROM pages WHERE quality_score > 0) as avg_quality_score,
                    (SELECT COUNT(*) FROM crawl_sessions) as crawl_sessions
                "#,
        )
        .fetch_one(pool)
        .await?;

        let _size_mb = Self::calculate_database_size(pool).await.unwrap_or(0.0);

        Ok(crate::storage::DatabaseStats {
            total_pages: row.get("total_pages"),
            total_links: row.get("total_links"),
            total_domains: row.get("total_domains"),
            avg_quality_score: row.get("avg_quality_score"),
            crawl_sessions: row.get("crawl_sessions"),
            database_size_mb: 0.0,
        })
    }

    // calculate approximate db size
    async fn calculate_database_size(pool: &DatabasePool) -> Result<f64> {
        let _row = sqlx::query("PRAGMA page_count; PRAGMA page_size;")
            .fetch_one(pool)
            .await?;

        // This is a simplified calculation - actual implementation would be more complex
        Ok(0.0) // Placeholder - would calculate from page_count * page_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn try_connect() -> Option<DatabasePool> {
        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://crawler_user:crawler_pass@localhost:5432/crawler_db".to_string()
        });
        let config = DatabaseConfig {
            database_url: url,
            max_connections: 5,
            enable_wal_mode: false,
            enable_foreign_keys: true,
        };
        Database::connect(&config).await.ok()
    }

    #[tokio::test]
    async fn test_database_connection() {
        if let Some(pool) = try_connect().await {
            assert!(Database::health_check(&pool).await);
        }
        // Passes silently when PostgreSQL is not available
    }

    #[tokio::test]
    async fn test_database_migrations() {
        let pool = match try_connect().await {
            Some(p) => p,
            None => return, // Skip when PostgreSQL is not available
        };

        // First run: applies all pending migrations
        Database::migrate(&pool)
            .await
            .expect("Migration should succeed on fresh or existing database");

        // Verify pages.tfidf_score was created by migration 005
        let tfidf_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
             WHERE table_name='pages' AND column_name='tfidf_score')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(tfidf_exists, "pages.tfidf_score must exist after migration");

        // Verify pages.pagerank was created by migration 004
        let pagerank_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
             WHERE table_name='pages' AND column_name='pagerank')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(pagerank_exists, "pages.pagerank must exist after migration");

        // Verify core tables exist
        for table in &["pages", "links", "crawl_sessions", "domains"] {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
                 WHERE table_name=$1)",
            )
            .bind(*table)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert!(exists, "Table {table} must exist after migration");
        }

        // Second run: idempotency check — must not error on already-applied migrations
        Database::migrate(&pool)
            .await
            .expect("Second migration run must succeed without duplicate-column errors");
    }
}
