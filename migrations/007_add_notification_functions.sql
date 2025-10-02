-- Add PostgreSQL functions and triggers for real-time notifications
-- These support LISTEN/NOTIFY functionality for event streaming

-- Function for bet notifications
CREATE OR REPLACE FUNCTION notify_bet_with_operation() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'bet_event',
        json_build_object(
            'operation', TG_OP,
            'bet_id', NEW.bet_id,
            'market_id', NEW.market_id,
            'user_addr', NEW.user_addr,
            'position', NEW.position,
            'amount', NEW.amount,
            'claimed', NEW.claimed,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function for blockchain event notifications
CREATE OR REPLACE FUNCTION notify_blockchain_event() RETURNS TRIGGER AS $$
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

-- Function for market resolution notifications
CREATE OR REPLACE FUNCTION notify_market_resolution() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'market_resolution_event',
        json_build_object(
            'market_id', NEW.market_id,
            'outcome', NEW.outcome,
            'total_yes_pool', NEW.total_yes_pool,
            'total_no_pool', NEW.total_no_pool,
            'total_yield_earned', NEW.total_yield_earned,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for bet notifications
DROP TRIGGER IF EXISTS bet_notification_trigger ON bets;
CREATE TRIGGER bet_notification_trigger
    AFTER INSERT OR UPDATE ON bets
    FOR EACH ROW
    EXECUTE FUNCTION notify_bet_with_operation();

-- Trigger for blockchain event notifications
DROP TRIGGER IF EXISTS blockchain_event_notification_trigger ON blockchain_events;
CREATE TRIGGER blockchain_event_notification_trigger
    AFTER INSERT ON blockchain_events
    FOR EACH ROW
    EXECUTE FUNCTION notify_blockchain_event();

-- Trigger for market resolution notifications
DROP TRIGGER IF EXISTS market_resolution_notification_trigger ON market_resolutions;
CREATE TRIGGER market_resolution_notification_trigger
    AFTER INSERT ON market_resolutions
    FOR EACH ROW
    EXECUTE FUNCTION notify_market_resolution();
