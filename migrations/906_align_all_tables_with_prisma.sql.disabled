-- Migration to align ALL tables with Prisma schema (camelCase naming)

-- ============================================================================
-- USERS TABLE
-- ============================================================================
ALTER TABLE users RENAME COLUMN avatar_url TO "avatarUrl";
ALTER TABLE users RENAME COLUMN created_at TO "createdAt";
ALTER TABLE users RENAME COLUMN updated_at TO "updatedAt";

DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- PROTOCOLS TABLE
-- ============================================================================
ALTER TABLE protocols RENAME COLUMN display_name TO "displayName";
ALTER TABLE protocols RENAME COLUMN base_apy TO "baseApy";
ALTER TABLE protocols RENAME COLUMN is_active TO "isActive";
ALTER TABLE protocols RENAME COLUMN icon_url TO "iconUrl";
ALTER TABLE protocols RENAME COLUMN created_at TO "createdAt";
ALTER TABLE protocols RENAME COLUMN updated_at TO "updatedAt";

DROP TRIGGER IF EXISTS update_protocols_updated_at ON protocols;
CREATE TRIGGER update_protocols_updated_at
    BEFORE UPDATE ON protocols
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- YIELD_RECORDS TABLE
-- ============================================================================
ALTER TABLE yield_records RENAME COLUMN market_id TO "marketId";
ALTER TABLE yield_records RENAME COLUMN protocol_id TO "protocolId";
ALTER TABLE yield_records RENAME COLUMN yield_amount TO "yield";
ALTER TABLE yield_records RENAME COLUMN created_at TO "createdAt";

-- Update indexes
DROP INDEX IF EXISTS idx_yield_records_market_id;
DROP INDEX IF EXISTS idx_yield_records_protocol_id;
DROP INDEX IF EXISTS idx_yield_records_period;

CREATE INDEX IF NOT EXISTS "idx_yield_records_marketId" ON yield_records("marketId");
CREATE INDEX IF NOT EXISTS "idx_yield_records_protocolId" ON yield_records("protocolId");
CREATE INDEX IF NOT EXISTS idx_yield_records_period ON yield_records(period);

-- Update foreign key constraints
ALTER TABLE yield_records DROP CONSTRAINT IF EXISTS yield_records_protocol_id_fkey;
ALTER TABLE yield_records ADD CONSTRAINT "yield_records_protocolId_fkey" 
    FOREIGN KEY ("protocolId") REFERENCES protocols(id);

-- ============================================================================
-- FEE_RECORDS TABLE
-- ============================================================================
ALTER TABLE fee_records RENAME COLUMN market_id TO "marketId";
ALTER TABLE fee_records RENAME COLUMN fee_type TO "feeType";
ALTER TABLE fee_records RENAME COLUMN created_at TO "createdAt";

-- Update indexes
DROP INDEX IF EXISTS idx_fee_records_market_id;
DROP INDEX IF EXISTS idx_fee_records_fee_type;

CREATE INDEX IF NOT EXISTS "idx_fee_records_marketId" ON fee_records("marketId");
CREATE INDEX IF NOT EXISTS "idx_fee_records_feeType" ON fee_records("feeType");

-- ============================================================================
-- BLOCKCHAIN_EVENTS TABLE
-- ============================================================================
ALTER TABLE blockchain_events RENAME COLUMN event_type TO "eventType";
ALTER TABLE blockchain_events RENAME COLUMN blockchain_id TO "blockchainId";
ALTER TABLE blockchain_events RENAME COLUMN block_number TO "blockNumber";
ALTER TABLE blockchain_events RENAME COLUMN block_timestamp TO "blockTimestamp";
ALTER TABLE blockchain_events RENAME COLUMN transaction_hash TO "transactionHash";
ALTER TABLE blockchain_events RENAME COLUMN created_at TO "createdAt";
ALTER TABLE blockchain_events RENAME COLUMN updated_at TO "updatedAt";

-- Update indexes and constraints
DROP INDEX IF EXISTS idx_blockchain_events_block_number;
DROP INDEX IF EXISTS idx_blockchain_events_type_processed;
ALTER TABLE blockchain_events DROP CONSTRAINT IF EXISTS blockchain_events_event_type_blockchain_id_key;

CREATE INDEX IF NOT EXISTS "idx_blockchain_events_blockNumber" ON blockchain_events("blockNumber");
CREATE INDEX IF NOT EXISTS "idx_blockchain_events_eventType_processed" ON blockchain_events("eventType", processed);
ALTER TABLE blockchain_events ADD CONSTRAINT "blockchain_events_eventType_blockchainId_key" 
    UNIQUE ("eventType", "blockchainId");

DROP TRIGGER IF EXISTS update_blockchain_events_updated_at ON blockchain_events;
CREATE TRIGGER update_blockchain_events_updated_at
    BEFORE UPDATE ON blockchain_events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- SYNC_STATUS TABLE
-- ============================================================================
ALTER TABLE sync_status RENAME COLUMN event_type TO "eventType";
ALTER TABLE sync_status RENAME COLUMN last_sync_block TO "lastSyncBlock";
ALTER TABLE sync_status RENAME COLUMN last_sync_time TO "lastSyncTime";
ALTER TABLE sync_status RENAME COLUMN is_active TO "isActive";
ALTER TABLE sync_status RENAME COLUMN created_at TO "createdAt";
ALTER TABLE sync_status RENAME COLUMN updated_at TO "updatedAt";

-- Update constraints
ALTER TABLE sync_status DROP CONSTRAINT IF EXISTS sync_status_event_type_key;
ALTER TABLE sync_status ADD CONSTRAINT "sync_status_eventType_key" UNIQUE ("eventType");

DROP TRIGGER IF EXISTS update_sync_status_updated_at ON sync_status;
CREATE TRIGGER update_sync_status_updated_at
    BEFORE UPDATE ON sync_status
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- BETS_EXTENDED TABLE (rename to bets to match Prisma)
-- ============================================================================
-- Note: We'll rename bets_extended to bets after backing up the indexer bets table

-- First, rename columns in bets_extended
ALTER TABLE bets_extended RENAME COLUMN bet_id TO "blockchainBetId";
ALTER TABLE bets_extended RENAME COLUMN user_id TO "userId";
ALTER TABLE bets_extended RENAME COLUMN created_at TO "createdAt";
ALTER TABLE bets_extended RENAME COLUMN updated_at TO "updatedAt";

-- Add missing columns to match Prisma Bet model
ALTER TABLE bets_extended ADD COLUMN IF NOT EXISTS "marketId" TEXT;
ALTER TABLE bets_extended ADD COLUMN IF NOT EXISTS position BOOLEAN;
ALTER TABLE bets_extended ADD COLUMN IF NOT EXISTS amount NUMERIC(78,18);

-- Update indexes
DROP INDEX IF EXISTS idx_bets_extended_bet_id;
DROP INDEX IF EXISTS idx_bets_extended_user_id;
DROP INDEX IF EXISTS idx_bets_extended_status;

CREATE INDEX IF NOT EXISTS "idx_bets_extended_blockchainBetId" ON bets_extended("blockchainBetId");
CREATE INDEX IF NOT EXISTS "idx_bets_extended_userId" ON bets_extended("userId");
CREATE INDEX IF NOT EXISTS "idx_bets_extended_marketId" ON bets_extended("marketId");
CREATE INDEX IF NOT EXISTS idx_bets_extended_status ON bets_extended(status);

-- Update constraints and foreign keys
ALTER TABLE bets_extended DROP CONSTRAINT IF EXISTS bets_extended_bet_id_key;
ALTER TABLE bets_extended DROP CONSTRAINT IF EXISTS bets_extended_bet_id_fkey;
ALTER TABLE bets_extended DROP CONSTRAINT IF EXISTS bets_extended_user_id_fkey;

ALTER TABLE bets_extended ADD CONSTRAINT "bets_extended_blockchainBetId_key" UNIQUE ("blockchainBetId");
ALTER TABLE bets_extended ADD CONSTRAINT "bets_extended_userId_fkey" 
    FOREIGN KEY ("userId") REFERENCES users(id);

DROP TRIGGER IF EXISTS update_bets_extended_updated_at ON bets_extended;
CREATE TRIGGER update_bets_extended_updated_at
    BEFORE UPDATE ON bets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- ADD COMMENTS
-- ============================================================================
COMMENT ON TABLE users IS 'Users table - aligned with Prisma schema';
COMMENT ON TABLE protocols IS 'Protocols table - aligned with Prisma schema';
COMMENT ON TABLE yield_records IS 'Yield records table - aligned with Prisma schema';
COMMENT ON TABLE fee_records IS 'Fee records table - aligned with Prisma schema';
COMMENT ON TABLE blockchain_events IS 'Blockchain events table - aligned with Prisma schema';
COMMENT ON TABLE sync_status IS 'Sync status table - aligned with Prisma schema';
COMMENT ON TABLE bets_extended IS 'Bets table - aligned with Prisma schema (camelCase columns)';