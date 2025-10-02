-- Add missing tables and columns that backend services expect

-- Add missing indexer tables
CREATE TABLE IF NOT EXISTS indexer_state (
    id SERIAL PRIMARY KEY,
    indexer_name TEXT NOT NULL UNIQUE,
    last_processed_version BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS event_processing_stats (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_count BIGINT NOT NULL DEFAULT 0,
    error_count BIGINT NOT NULL DEFAULT 0,
    last_processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Add missing columns to bets table for yield calculation
ALTER TABLE bets ADD COLUMN IF NOT EXISTS claimed BOOLEAN DEFAULT FALSE;

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_indexer_state_name ON indexer_state(indexer_name);
CREATE INDEX IF NOT EXISTS idx_event_stats_type ON event_processing_stats(event_type);
CREATE INDEX IF NOT EXISTS idx_bets_claimed ON bets(claimed);
