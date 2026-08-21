//! Repository pattern for database operations

use crate::models::CrawlUrl;
use crate::models::PageData;
use crate::storage::models::{CrawlSession, DatabaseStats, PageFilter, StoredPage};
use crate::storage::{Result, StorageError};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use tracing::info;

pub struct PageRepository {
    pool: PgPool,
}

impl PageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn calculate_url_hash(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let bytes = hasher.finalize();
        hex::encode(bytes)
    }

    fn calculate_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let bytes = hasher.finalize();
        hex::encode(bytes)
    }

    pub async fn save_page(&self, page: &PageData, _session_id: i64) -> Result<i64> {
        let url_hash = Self::calculate_url_hash(&page.url);
        let content_hash = Self::calculate_content_hash(&page.content);
        let stored_page = StoredPage::from_page_data(page, url_hash, content_hash);

        let query = r#"
            INSERT INTO pages (
                url, url_hash, domain, title, description, content, content_hash,
                quality_score, word_count, language, crawl_depth, crawled_at,
                status_code, content_type, content_length
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (url_hash)
            DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                content = EXCLUDED.content,
                content_hash = EXCLUDED.content_hash,
                quality_score = EXCLUDED.quality_score,
                word_count = EXCLUDED.word_count,
                crawled_at = EXCLUDED.crawled_at,
                status_code = EXCLUDED.status_code,
                content_length = EXCLUDED.content_length
            RETURNING id
        "#;

        //  CHANGE 4: Use fetch_one instead of execute to get RETURNING value
        let row = sqlx::query(query)
            .bind(&stored_page.url)
            .bind(&stored_page.url_hash)
            .bind(&stored_page.domain)
            .bind(&stored_page.title)
            .bind(&stored_page.description)
            .bind(&stored_page.content)
            .bind(&stored_page.content_hash)
            .bind(stored_page.quality_score)
            .bind(stored_page.word_count as i32)
            .bind(&stored_page.language)
            .bind(stored_page.crawl_depth as i32)
            .bind(stored_page.crawled_at)
            .bind(stored_page.status_code as i32)
            .bind(&stored_page.content_type)
            .bind(stored_page.content_length as i32)
            .fetch_one(&self.pool)
            .await?;

        let page_id: i64 = row.get("id");

        self.update_domain_stats(&stored_page.domain, stored_page.quality_score)
            .await?;

        info!("Saved page: {} (ID: {})", page.url, page_id);
        Ok(page_id)
    }

    pub async fn save_links(&self, page_id: i64, links: &[CrawlUrl]) -> Result<()> {
        if links.is_empty() {
            return Ok(());
        }

        //  CHANGE: Use $1, $2 and ON CONFLICT
        let query = r#"
            INSERT INTO links (source_page_id, target_url, anchor_text, link_position)
            VALUES ($1,
            (SELECT id FROM pages WHERE url = $2 LIMIT 1),
             $2, $3, $4)
            ON CONFLICT DO NOTHING
        "#;

        for (position, link) in links.iter().enumerate() {
            let anchor_text: Option<String> = None;
            sqlx::query(query)
                .bind(page_id)
                .bind(&link.url)
                .bind(anchor_text)
                .bind(position as i32)
                .execute(&self.pool)
                .await?;
        }

        info!("🔗 Saved {} links for page ID {}", links.len(), page_id);
        Ok(())
    }

    pub async fn get_page_by_id(&self, page_id: i64) -> Result<Option<StoredPage>> {
        //  CHANGE: Use $1 instead of ?
        let query = r#"
            SELECT id, url, url_hash, domain, title, description, content, content_hash,
                   quality_score, word_count, language, crawl_depth, crawled_at, last_modified,
                   status_code, content_type, content_length
            FROM pages WHERE id = $1
        "#;

        let page = sqlx::query_as::<_, StoredPage>(query)
            .bind(page_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(page)
    }

    pub async fn get_page_by_url(&self, url: &str) -> Result<Option<StoredPage>> {
        let url_hash = Self::calculate_url_hash(url);

        //  CHANGE: Use $1 instead of ?
        let query = r#"
            SELECT id, url, url_hash, domain, title, description, content, content_hash,
                   quality_score, word_count, language, crawl_depth, crawled_at, last_modified,
                   status_code, content_type, content_length
            FROM pages WHERE url_hash = $1
        "#;

        let page = sqlx::query_as::<_, StoredPage>(query)
            .bind(&url_hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(page)
    }

    pub async fn url_exists(&self, url: &str) -> Result<bool> {
        let url_hash = Self::calculate_url_hash(url);
        //  CHANGE: Use $1 instead of ?
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE url_hash = $1")
            .bind(&url_hash)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn get_pages(&self, filter: &PageFilter) -> Result<Vec<StoredPage>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT id, url, url_hash, domain, title, description, content, content_hash, \
             quality_score, word_count, language, crawl_depth, crawled_at, last_modified, \
             status_code, content_type, content_length, pagerank, tfidf_score FROM pages WHERE 1=1",
        );

        if let Some(domain) = &filter.domain {
            qb.push(" AND domain = ").push_bind(domain);
        }
        if let Some(min_q) = filter.min_quality {
            qb.push(" AND quality_score >= ").push_bind(min_q);
        }
        if let Some(max_q) = filter.max_quality {
            qb.push(" AND quality_score <= ").push_bind(max_q);
        }
        if let Some(sc) = filter.status_code {
            qb.push(" AND status_code = ").push_bind(sc);
        }
        if let Some(after) = &filter.crawled_after {
            qb.push(" AND crawled_at >= ").push_bind(after.to_rfc3339());
        }
        if let Some(before) = &filter.crawled_before {
            qb.push(" AND crawled_at <= ")
                .push_bind(before.to_rfc3339());
        }

        qb.push(" ORDER BY quality_score DESC, crawled_at DESC");

        if let Some(limit) = filter.limit {
            qb.push(" LIMIT ").push_bind(limit as i64);
            if let Some(offset) = filter.offset {
                qb.push(" OFFSET ").push_bind(offset as i64);
            }
        }

        let query = qb.build_query_as::<StoredPage>();
        Ok(query.fetch_all(&self.pool).await?)
    }

    pub async fn get_all_links(&self) -> Result<Vec<(String, String)>> {
        let sql = r#"
        SELECT DISTINCT p1.url as source_url, l.target_url as target_url
        FROM links l
        INNER JOIN pages p1 ON l.source_page_id = p1.id
        INNER JOIN pages p2 ON l.target_url = p2.url
    "#;

        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;

        let mut links = Vec::new();

        for row in rows {
            // Use get() instead of try_get() - SQLx handles the types automatically
            let source: String = row.get("source_url");
            let target: String = row.get("target_url");
            links.push((source, target));
        }

        Ok(links)
    }

    // update page rank values for a page
    pub async fn update_pagerank(&self, url: &str, pagerank: f64) -> Result<()> {
        let url_hash = Self::calculate_url_hash(url);

        let query = r#"
            UPDATE pages
            SET pagerank = $1
            WHERE url_hash = $2
        "#;

        sqlx::query(query)
            .bind(pagerank)
            .bind(&url_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // batch update pagerank values
    pub async fn batch_update_pagerank(&self, ranks: &[(String, f64)]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let query = r#"
            UPDATE pages
            SET pagerank = $1
            WHERE url_hash = $2
        "#;

        for (url, rank) in ranks {
            let url_hash = Self::calculate_url_hash(url);
            sqlx::query(query)
                .bind(rank)
                .bind(url_hash)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        info!("Batch updated {} PageRank values", ranks.len());
        Ok(())
    }

    // get pages with highest PageRank
    pub async fn get_top_pages_by_pagerank(&self, limit: usize) -> Result<Vec<StoredPage>> {
        let query = r#"
            SELECT id, url, url_hash, domain, title, description, content, content_hash,
            quality_score, word_count, language, crawl_depth, crawled_at, last_modified,
            status_code, content_type, content_length

            FROM pages
            WHERE pagerank is not NULL
            ORDER BY pagerank DESC
            LIMIT $1
        "#;

        let pages = sqlx::query_as::<_, StoredPage>(query)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;

        Ok(pages)
    }

    pub async fn get_pages_by_domain(&self, domain: &str, limit: usize) -> Result<Vec<StoredPage>> {
        let filter = PageFilter::new()
            .with_domain(domain.to_string())
            .with_limit(limit);
        self.get_pages(&filter).await
    }

    pub async fn search_pages(&self, q: &str, limit: usize) -> Result<Vec<StoredPage>> {
        let like = format!("%{}%", q);
        //  CHANGE: Use $1, $2, $3 instead of ?
        let sql = r#"
            SELECT id, url, url_hash, domain, title, description, content, content_hash,
                   quality_score, word_count, language, crawl_depth, crawled_at, last_modified,
                   status_code, content_type, content_length
            FROM pages
            WHERE title LIKE $1 OR description LIKE $2 OR content LIKE $3
            ORDER BY quality_score DESC
            LIMIT $4
        "#;

        let pages = sqlx::query_as::<_, StoredPage>(sql)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;

        Ok(pages)
    }

    pub async fn batch_save_pages(&self, pages: &[PageData], _session_id: i64) -> Result<Vec<i64>> {
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::new();

        for page in pages {
            let url_hash = Self::calculate_url_hash(&page.url);
            let content_hash = Self::calculate_content_hash(&page.content);
            let stored_page = StoredPage::from_page_data(page, url_hash, content_hash);

            //  CHANGE: PostgreSQL syntax + RETURNING
            let query = r#"
                INSERT INTO pages (
                    url, url_hash, domain, title, description, content, content_hash,
                    quality_score, word_count, language, crawl_depth, crawled_at,
                    status_code, content_type, content_length
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (url_hash) DO UPDATE SET
                    content = EXCLUDED.content,
                    quality_score = EXCLUDED.quality_score
                RETURNING id
            "#;

            let row = sqlx::query(query)
                .bind(&stored_page.url)
                .bind(&stored_page.url_hash)
                .bind(&stored_page.domain)
                .bind(&stored_page.title)
                .bind(&stored_page.description)
                .bind(&stored_page.content)
                .bind(&stored_page.content_hash)
                .bind(stored_page.quality_score)
                .bind(stored_page.word_count as i32)
                .bind(&stored_page.language)
                .bind(stored_page.crawl_depth as i32)
                .bind(stored_page.crawled_at)
                .bind(stored_page.status_code as i32)
                .bind(&stored_page.content_type)
                .bind(stored_page.content_length as i32)
                .fetch_one(&mut *tx)
                .await?;

            ids.push(row.get("id"));
        }

        tx.commit().await?;
        info!("📦 Batch saved {} pages", ids.len());
        Ok(ids)
    }

    pub async fn update_tfidf_score(&self, url_hash: &str, tfidf: f64) -> Result<()> {
        sqlx::query("UPDATE pages SET tfidf_score = $1 WHERE url_hash = $2")
            .bind(tfidf)
            .bind(url_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_domain_stats(&self, domain: &str, _quality_score: f64) -> Result<()> {
        //  CHANGE: PostgreSQL upsert syntax
        let query = r#"
            INSERT INTO domains (domain, page_count, avg_quality_score, last_crawled)
            VALUES (
                $1,
                1,
                (SELECT AVG(quality_score) FROM pages WHERE domain = $2),
                CURRENT_TIMESTAMP
            )
            ON CONFLICT (domain) DO UPDATE SET
                page_count = domains.page_count + 1,
                avg_quality_score = (SELECT AVG(quality_score) FROM pages WHERE domain = $3),
                last_crawled = CURRENT_TIMESTAMP
        "#;

        sqlx::query(query)
            .bind(domain)
            .bind(domain)
            .bind(domain)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_crawl_session(
        &self,
        seed_urls: &[String],
        config: &crate::config::CrawlerConfig,
    ) -> Result<i64> {
        let session = CrawlSession::new(seed_urls, config)?;
        //  CHANGE: Use $1, $2, $3 and RETURNING
        let query = r#"
            INSERT INTO crawl_sessions (started_at, seed_urls, config_snapshot, status)
            VALUES ($1, $2, $3, $4)
            RETURNING id
        "#;

        let row = sqlx::query(query)
            .bind(session.started_at)
            .bind(&session.seed_urls)
            .bind(&session.config_snapshot)
            .bind(&session.status)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("id"))
    }

    pub async fn update_crawl_session(
        &self,
        session_id: i64,
        crawled: i32,
        failed: i32,
    ) -> Result<()> {
        //  CHANGE: Use $1, $2, $3
        let query = r#"
            UPDATE crawl_sessions
            SET pages_crawled = $1, pages_failed = $2
            WHERE id = $3
        "#;

        sqlx::query(query)
            .bind(crawled)
            .bind(failed)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn complete_crawl_session(&self, session_id: i64, status: &str) -> Result<()> {
        //  CHANGE: Use $1, $2
        let query = r#"
            UPDATE crawl_sessions
            SET ended_at = CURRENT_TIMESTAMP, status = $1
            WHERE id = $2
        "#;

        sqlx::query(query)
            .bind(status)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM pages) as total_pages,
                (SELECT COUNT(*) FROM links) as total_links,
                (SELECT COUNT(*) FROM domains) as total_domains,
                (SELECT AVG(quality_score) FROM pages WHERE quality_score > 0) as avg_quality_score,
                (SELECT COUNT(*) FROM crawl_sessions) as crawl_sessions
        "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DatabaseStats {
            total_pages: row.get("total_pages"),
            total_links: row.get("total_links"),
            total_domains: row.get("total_domains"),
            avg_quality_score: row.get("avg_quality_score"),
            crawl_sessions: row.get("crawl_sessions"),
            database_size_mb: 0.0,
        })
    }
}
