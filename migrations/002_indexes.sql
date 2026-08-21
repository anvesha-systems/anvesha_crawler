CREATE INDEX IF NOT EXISTS idx_pages_content_hash ON pages(content_hash);
CREATE INDEX IF NOT EXISTS idx_pages_status_code ON pages(status_code);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON crawl_sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON crawl_sessions(status);
