-- Migration: Add UPDATE triggers for all entities to detect changes
-- This allows the backend to react to ANY changes in the indexer database

-- =====================================================
-- 1. Add UPDATE triggers for bets
-- =====================================================

-- Trigger for bet updates (status changes, claims, etc.)
DROP TRIGGER IF EXISTS trigger_notify_bet_update ON bets;
CREATE TRIGGER trigger_notify_bet_update
    AFTER UPDATE ON bets
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_new_bet();

-- =====================================================
-- 2. Add UPDATE triggers for markets
-- =====================================================

-- Trigger for market updates (resolution, status changes, etc.)
DROP TRIGGER IF EXISTS trigger_notify_market_update ON markets;
CREATE TRIGGER trigger_notify_market_update
    AFTER UPDATE ON markets
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_new_market();

-- =====================================================
-- 3. Add UPDATE triggers for market resolutions
-- =====================================================

DROP TRIGGER IF EXISTS trigger_notify_market_resolution_update ON market_resolutions;
CREATE TRIGGER trigger_notify_market_resolution_update
    AFTER UPDATE ON market_resolutions
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_market_resolution();

-- =====================================================
-- 4. Add UPDATE triggers for winnings claims
-- =====================================================

DROP TRIGGER IF EXISTS trigger_notify_winnings_claim_update ON winnings_claims;
CREATE TRIGGER trigger_notify_winnings_claim_update
    AFTER UPDATE ON winnings_claims
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_winnings_claim();

-- =====================================================
-- 5. Add UPDATE triggers for yield deposits
-- =====================================================

DROP TRIGGER IF EXISTS trigger_notify_yield_deposit_update ON yield_deposits;
CREATE TRIGGER trigger_notify_yield_deposit_update
    AFTER UPDATE ON yield_deposits
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_yield_deposit();

-- =====================================================
-- 6. Add UPDATE triggers for protocol fees
-- =====================================================

DROP TRIGGER IF EXISTS trigger_notify_protocol_fee_update ON protocol_fees;
CREATE TRIGGER trigger_notify_protocol_fee_update
    AFTER UPDATE ON protocol_fees
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_protocol_fee();

-- =====================================================
-- 7. Add UPDATE triggers for blockchain events
-- =====================================================

DROP TRIGGER IF EXISTS trigger_notify_blockchain_event_update ON blockchain_events;
CREATE TRIGGER trigger_notify_blockchain_event_update
    AFTER UPDATE ON blockchain_events
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION notify_blockchain_event();

-- =====================================================
-- 8. Create enhanced notification functions with operation type
-- =====================================================

-- Enhanced bet notification with operation type
CREATE OR REPLACE FUNCTION notify_bet_with_operation()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'bet_event',
        json_build_object(
            'operation', TG_OP,  -- 'INSERT' or 'UPDATE'
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

-- Enhanced market notification with operation type
CREATE OR REPLACE FUNCTION notify_market_with_operation()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'market_event',
        json_build_object(
            'operation', TG_OP,  -- 'INSERT' or 'UPDATE'
            'market_id', NEW.market_id,
            'question', NEW.question,
            'end_time', NEW.end_time,
            'resolved', NEW.resolved,
            'outcome', NEW.outcome,
            'yield_protocol_addr', NEW.yield_protocol_addr,
            'transaction_version', NEW.transaction_version
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- =====================================================
-- 9. Replace existing triggers with enhanced versions
-- =====================================================

-- Replace bet triggers
DROP TRIGGER IF EXISTS trigger_notify_new_bet ON bets;
DROP TRIGGER IF EXISTS trigger_notify_bet_update ON bets;
CREATE TRIGGER trigger_notify_bet_changes
    AFTER INSERT OR UPDATE ON bets
    FOR EACH ROW
    EXECUTE FUNCTION notify_bet_with_operation();

-- Replace market triggers
DROP TRIGGER IF EXISTS trigger_notify_new_market ON markets;
DROP TRIGGER IF EXISTS trigger_notify_market_update ON markets;
CREATE TRIGGER trigger_notify_market_changes
    AFTER INSERT OR UPDATE ON markets
    FOR EACH ROW
    EXECUTE FUNCTION notify_market_with_operation();

-- =====================================================
-- 10. Add notification channels view for monitoring
-- =====================================================

CREATE OR REPLACE VIEW active_notification_channels AS
SELECT 
    'bet_event' as channel_name,
    'Notifies on bet INSERT/UPDATE' as description
UNION ALL
SELECT 
    'market_event' as channel_name,
    'Notifies on market INSERT/UPDATE' as description
UNION ALL
SELECT 
    'new_bet_event' as channel_name,
    'Legacy: Notifies on new bets only' as description
UNION ALL
SELECT 
    'new_market_event' as channel_name,
    'Legacy: Notifies on new markets only' as description
UNION ALL
SELECT 
    'market_resolution_event' as channel_name,
    'Notifies on market resolutions' as description
UNION ALL
SELECT 
    'winnings_claim_event' as channel_name,
    'Notifies on winnings claims' as description
UNION ALL
SELECT 
    'yield_deposit_event' as channel_name,
    'Notifies on yield deposits' as description
UNION ALL
SELECT 
    'protocol_fee_event' as channel_name,
    'Notifies on protocol fees' as description
UNION ALL
SELECT 
    'blockchain_event' as channel_name,
    'Catch-all for blockchain events' as description;

COMMENT ON VIEW active_notification_channels IS 'Lists all active PostgreSQL NOTIFY channels for event monitoring';

-- =====================================================
-- 11. Test the triggers
-- =====================================================

-- You can test with:
-- UPDATE bets SET amount = amount WHERE bet_id = 1;
-- This should trigger a notification on 'bet_event' channel
