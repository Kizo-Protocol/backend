-- Add PostgreSQL functions and triggers for real-time notifications
-- These support LISTEN/NOTIFY functionality for event streaming

-- Function for blockchain event notifications
CREATE OR REPLACE FUNCTION notify_blockchain_event() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'blockchain_event',
        json_build_object(
            'id', NEW.id,
            'eventType', NEW."eventType",
            'blockchainId', NEW."blockchainId",
            'blockNumber', NEW."blockNumber",
            'processed', NEW.processed,
            'createdAt', NEW."createdAt"
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for blockchain event notifications on insert
DROP TRIGGER IF EXISTS trigger_notify_blockchain_event ON blockchain_events;
CREATE TRIGGER trigger_notify_blockchain_event
    AFTER INSERT ON blockchain_events
    FOR EACH ROW
    EXECUTE FUNCTION notify_blockchain_event();

-- Trigger for blockchain event notifications on update
DROP TRIGGER IF EXISTS trigger_notify_blockchain_event_update ON blockchain_events;
CREATE TRIGGER trigger_notify_blockchain_event_update
    AFTER UPDATE ON blockchain_events
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_blockchain_event();

-- Function for updating updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column() RETURNS TRIGGER AS $$
BEGIN
    NEW."updatedAt" = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add updated_at triggers to tables that need them
DROP TRIGGER IF EXISTS update_bets_extended_updated_at ON bets_extended;
CREATE TRIGGER update_bets_extended_updated_at
    BEFORE UPDATE ON bets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_blockchain_events_updated_at ON blockchain_events;
CREATE TRIGGER update_blockchain_events_updated_at
    BEFORE UPDATE ON blockchain_events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_sync_status_updated_at ON sync_status;
CREATE TRIGGER update_sync_status_updated_at
    BEFORE UPDATE ON sync_status
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
