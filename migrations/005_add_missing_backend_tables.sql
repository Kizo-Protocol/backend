-- Add missing tables that backend services expect
-- These are support tables for indexer state and event processing

-- Indexer state tracking
CREATE TABLE IF NOT EXISTS indexer_state (
    id SERIAL PRIMARY KEY,
    indexer_name TEXT NOT NULL UNIQUE,
    last_processed_version BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_indexer_state_name ON indexer_state(indexer_name);

-- Event processing statistics
CREATE TABLE IF NOT EXISTS event_processing_stats (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    total_processed BIGINT DEFAULT 0,
    successful BIGINT DEFAULT 0,
    errors BIGINT DEFAULT 0,
    avg_duration_ms NUMERIC(10, 2) DEFAULT 0,
    last_processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_event_stats_type ON event_processing_stats(event_type);
