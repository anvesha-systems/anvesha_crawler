mod bloom_filter;
mod logging;
mod metrics;
mod url_utils;
pub use bloom_filter::*;
pub use logging::*;
pub use url_utils::*;

// Utility functions for the crawler

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging for the application
pub fn init_logging() -> crate::Result<()> {
    // Check if already initialized to make it idempotent
    static INIT: std::sync::Once = std::sync::Once::new();
    static mut INITIALIZED: bool = false;

    INIT.call_once(|| {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "crawler=info,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        unsafe {
            INITIALIZED = true;
        }
    });

    Ok(())
}

/// Initialize metrics collection (placeholder for future implementation)
pub async fn init_metrics() -> crate::Result<()> {
    // Placeholder for future metrics implementation (Prometheus, etc.)
    // For now, just return Ok
    tracing::info!("Metrics system initialized (placeholder)");
    Ok(())
}

/// Initialize both logging and metrics
pub async fn init() -> crate::Result<()> {
    init_logging()?;
    init_metrics().await?;
    Ok(())
}

#[cfg(test)]
mod tests;
