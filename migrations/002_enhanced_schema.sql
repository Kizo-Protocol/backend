-- Additional Schema Migration for Kizo Server
-- Only contains tables/modifications not in migration 1

-- Event processing log table (not in migration 1)
CREATE TABLE IF NOT EXISTS event_processing_log (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    transaction_version BIGINT,
    processing_status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    processing_duration_ms INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_event_processing_log_event_type ON event_processing_log(event_type);
CREATE INDEX IF NOT EXISTS idx_event_processing_log_status ON event_processing_log(processing_status);
CREATE INDEX IF NOT EXISTS idx_event_processing_log_created_at ON event_processing_log(created_at);
