/*

use serde::{ Serialize, Deserialize };
use sqlx::postgres::PgConnectOptions;
use sqlx::{ ConnectOptions, PgPool, Pool, Postgres, };
use std::str::FromStr;
use std::time::Duration;
use clap::builder::styling::AnsiColor::Red;
use tracing::log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub migrate_on_startup: bool,
    pub redis : Option<RedisConfig>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub command_timeout: u64,
}


impl DatabaseConfig {
    pub fn new() -> Self{
        Self{
            url : "postgres://crawler_user:crawler_password@localhost:5432/webcrawler".to_string(),
            max_connections : 100,
            min_connections : 5,
            connect_timeout : 30,
            idle_timeout : 600,
            max_lifetime : 3600,
            migrate_on_startup : true,
            redis : Some(RedisConfig{
                url : "redis://localhost:6379".to_string(),
                pool_size : 20,
                connection_timeout : 5,
                command_timeout : 10
            }),
        }
    }

    // create  database connection pool
    pub async fn create_pool(&self) -> Result<PgPool, sqlx::Error>{
        let mut options = PgConnectOptions::from_str(&self.url)?
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Warn, Duration::from_millis(1000));

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connect_timeout))
            .idle_timeout(Duration::from_secs(self.idle_timeout))
            .max_lifetime(Duration::from_secs(self.max_lifetime))
            .connect_with(options)
            .await?;

        if self.migrate_on_startup {
            sqlx::migrate!("./migrations").run(&pool).await?;
        }

        Ok(pool)
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::new()
    }
}

 */

use serde::{Deserialize, Serialize};
#[cfg(feature = "database")]
use sqlx::postgres::PgConnectOptions;
#[cfg(feature = "database")]
use sqlx::{ConnectOptions, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub migrate_on_startup: bool,
}

impl DatabaseConfig {
    pub fn new() -> Self {
        Self {
            url: "postgresql://localhost/crawler".to_string(),
            max_connections: 100,
            min_connections: 5,
            connect_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
            migrate_on_startup: false, // Disable for now
        }
    }

    #[cfg(feature = "database")]
    pub async fn create_pool(&self) -> Result<PgPool, sqlx::Error> {
        let options = PgConnectOptions::from_str(&self.url)?;

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connect_timeout))
            .idle_timeout(Duration::from_secs(self.idle_timeout))
            .max_lifetime(Duration::from_secs(self.max_lifetime))
            .connect_with(options)
            .await?;

        if self.migrate_on_startup {
            sqlx::migrate!("./migrations").run(&pool).await?;
        }

        Ok(pool)
    }

    #[cfg(not(feature = "database"))]
    pub async fn create_pool(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Placeholder when database feature is disabled
        Ok(())
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::new()
    }
}
