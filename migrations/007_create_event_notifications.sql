-- Migration: Create database triggers for real-time event notifications
-- This allows the backend to automatically react to ANY new events in the indexer

-- =====================================================
-- 1. Create notification functions
-- =====================================================

-- Function to notify on new bets
CREATE OR REPLACE FUNCTION notify_new_bet()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'new_bet_event',
        json_build_object(
            'bet_id', NEW.bet_id,
            'market_id', NEW.market_id,
            'user_addr', NEW.user_addr,
            'position', NEW.position,
            'amount', NEW.amount,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to notify on new markets
CREATE OR REPLACE FUNCTION notify_new_market()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'new_market_event',
        json_build_object(
            'market_id', NEW.market_id,
            'question', NEW.question,
            'end_time', NEW.end_time,
            'yield_protocol_addr', NEW.yield_protocol_addr,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to notify on market resolutions
CREATE OR REPLACE FUNCTION notify_market_resolution()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'market_resolution_event',
        json_build_object(
            'market_id', NEW.market_id,
            'outcome', NEW.outcome,
            'total_yes_pool', NEW.total_yes_pool,
            'total_no_pool', NEW.total_no_pool,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to notify on winnings claims
CREATE OR REPLACE FUNCTION notify_winnings_claim()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'winnings_claim_event',
        json_build_object(
            'bet_id', NEW.bet_id,
            'user_addr', NEW.user_addr,
            'winning_amount', NEW.winning_amount,
            'yield_share', NEW.yield_share,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to notify on yield deposits
CREATE OR REPLACE FUNCTION notify_yield_deposit()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'yield_deposit_event',
        json_build_object(
            'market_id', NEW.market_id,
            'amount', NEW.amount,
            'protocol_addr', NEW.protocol_addr,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to notify on protocol fee collections
CREATE OR REPLACE FUNCTION notify_protocol_fee()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'protocol_fee_event',
        json_build_object(
            'market_id', NEW.market_id,
            'fee_amount', NEW.fee_amount,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to notify on ANY blockchain event
CREATE OR REPLACE FUNCTION notify_blockchain_event()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'blockchain_event',
        json_build_object(
            'event_type', NEW.event_type,
            'market_id', NEW.market_id,
            'transaction_version', NEW.transaction_version,
            'event_data', NEW.event_data,
            'created_at', NEW.created_at
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- =====================================================
-- 2. Create triggers on indexer tables
-- =====================================================

-- Trigger for new bets
DROP TRIGGER IF EXISTS trigger_notify_new_bet ON bets;
CREATE TRIGGER trigger_notify_new_bet
    AFTER INSERT ON bets
    FOR EACH ROW
    EXECUTE FUNCTION notify_new_bet();

-- Trigger for new markets
DROP TRIGGER IF EXISTS trigger_notify_new_market ON markets;
CREATE TRIGGER trigger_notify_new_market
    AFTER INSERT ON markets
    FOR EACH ROW
    EXECUTE FUNCTION notify_new_market();

-- Trigger for market resolutions
DROP TRIGGER IF EXISTS trigger_notify_market_resolution ON market_resolutions;
CREATE TRIGGER trigger_notify_market_resolution
    AFTER INSERT ON market_resolutions
    FOR EACH ROW
    EXECUTE FUNCTION notify_market_resolution();

-- Trigger for winnings claims
DROP TRIGGER IF EXISTS trigger_notify_winnings_claim ON winnings_claims;
CREATE TRIGGER trigger_notify_winnings_claim
    AFTER INSERT ON winnings_claims
    FOR EACH ROW
    EXECUTE FUNCTION notify_winnings_claim();

-- Trigger for yield deposits
DROP TRIGGER IF EXISTS trigger_notify_yield_deposit ON yield_deposits;
CREATE TRIGGER trigger_notify_yield_deposit
    AFTER INSERT ON yield_deposits
    FOR EACH ROW
    EXECUTE FUNCTION notify_yield_deposit();

-- Trigger for protocol fees
DROP TRIGGER IF EXISTS trigger_notify_protocol_fee ON protocol_fees;
CREATE TRIGGER trigger_notify_protocol_fee
    AFTER INSERT ON protocol_fees
    FOR EACH ROW
    EXECUTE FUNCTION notify_protocol_fee();

-- Trigger for blockchain events (catch-all)
DROP TRIGGER IF EXISTS trigger_notify_blockchain_event ON blockchain_events;
CREATE TRIGGER trigger_notify_blockchain_event
    AFTER INSERT ON blockchain_events
    FOR EACH ROW
    EXECUTE FUNCTION notify_blockchain_event();

-- =====================================================
-- 3. Create event log table for audit trail
-- =====================================================

CREATE TABLE IF NOT EXISTS event_processing_log (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    transaction_version BIGINT,
    processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    processing_status VARCHAR(50) DEFAULT 'success',
    error_message TEXT,
    processing_duration_ms INTEGER
);

CREATE INDEX idx_event_log_type ON event_processing_log(event_type);
CREATE INDEX idx_event_log_transaction ON event_processing_log(transaction_version);
CREATE INDEX idx_event_log_processed_at ON event_processing_log(processed_at);

-- =====================================================
-- 4. Create event processing status view
-- =====================================================

CREATE OR REPLACE VIEW event_processing_stats AS
SELECT 
    event_type,
    COUNT(*) as total_processed,
    COUNT(CASE WHEN processing_status = 'success' THEN 1 END) as successful,
    COUNT(CASE WHEN processing_status = 'error' THEN 1 END) as errors,
    AVG(processing_duration_ms) as avg_duration_ms,
    MAX(processed_at) as last_processed_at
FROM event_processing_log
WHERE processed_at > NOW() - INTERVAL '1 hour'
GROUP BY event_type
ORDER BY total_processed DESC;

COMMENT ON VIEW event_processing_stats IS 'Real-time statistics for event processing in the last hour';
