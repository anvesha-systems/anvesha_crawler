-- Widen pagerank from REAL (FLOAT4) to DOUBLE PRECISION (FLOAT8) to match
-- the Rust model StoredPage.pagerank: Option<f64>.
-- All existing REAL values are exactly representable in DOUBLE PRECISION.
ALTER TABLE pages ALTER COLUMN pagerank TYPE DOUBLE PRECISION;
