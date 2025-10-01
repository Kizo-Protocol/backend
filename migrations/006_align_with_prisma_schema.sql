-- Migration to align PostgreSQL schema with Prisma schema
-- This aligns all field names with Prisma camelCase naming convention
-- Note: Keeping table as markets_extended (not renaming to markets to avoid conflict with indexer table)

-- Step 1: Rename columns to match Prisma camelCase naming
ALTER TABLE markets_extended RENAME COLUMN adj_ticker TO "adjTicker";
ALTER TABLE markets_extended RENAME COLUMN adj_market_id TO "marketId";
ALTER TABLE markets_extended RENAME COLUMN blockchain_market_id TO "blockchainMarketId";
ALTER TABLE markets_extended RENAME COLUMN end_date TO "endDate";
ALTER TABLE markets_extended RENAME COLUMN resolution_date TO "resolutionDate";
ALTER TABLE markets_extended RENAME COLUMN image_url TO "imageUrl";
ALTER TABLE markets_extended RENAME COLUMN open_interest TO "openInterest";
ALTER TABLE markets_extended RENAME COLUMN total_pool_size TO "totalPoolSize";
ALTER TABLE markets_extended RENAME COLUMN yes_pool_size TO "yesPoolSize";
ALTER TABLE markets_extended RENAME COLUMN no_pool_size TO "noPoolSize";
ALTER TABLE markets_extended RENAME COLUMN count_yes TO "countYes";
ALTER TABLE markets_extended RENAME COLUMN count_no TO "countNo";
ALTER TABLE markets_extended RENAME COLUMN current_yield TO "currentYield";
ALTER TABLE markets_extended RENAME COLUMN total_yield_earned_decimal TO "totalYieldEarned";
ALTER TABLE markets_extended RENAME COLUMN created_at TO "createdAt";
ALTER TABLE markets_extended RENAME COLUMN updated_at TO "updatedAt";

-- Step 2: Update indexes to match new column names
DROP INDEX IF EXISTS idx_markets_extended_adj_market_id;
DROP INDEX IF EXISTS idx_markets_extended_adj_ticker;
DROP INDEX IF EXISTS idx_markets_extended_blockchain_market_id;
DROP INDEX IF EXISTS idx_markets_extended_end_date;
DROP INDEX IF EXISTS idx_markets_extended_status;

CREATE INDEX IF NOT EXISTS "idx_markets_extended_marketId" ON markets_extended("marketId");
CREATE INDEX IF NOT EXISTS "idx_markets_extended_adjTicker" ON markets_extended("adjTicker");
CREATE INDEX IF NOT EXISTS "idx_markets_extended_blockchainMarketId" ON markets_extended("blockchainMarketId") WHERE "blockchainMarketId" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_markets_extended_endDate" ON markets_extended("endDate");
CREATE INDEX IF NOT EXISTS idx_markets_extended_status ON markets_extended(status);

-- Step 3: Update unique constraints to match new column names
ALTER TABLE markets_extended DROP CONSTRAINT IF EXISTS markets_extended_adj_market_id_key;
ALTER TABLE markets_extended DROP CONSTRAINT IF EXISTS markets_extended_adj_ticker_key;
ALTER TABLE markets_extended DROP CONSTRAINT IF EXISTS markets_extended_blockchain_market_id_key;
ALTER TABLE markets_extended DROP CONSTRAINT IF EXISTS markets_extended_pkey;

ALTER TABLE markets_extended ADD CONSTRAINT markets_extended_pkey PRIMARY KEY (id);
ALTER TABLE markets_extended ADD CONSTRAINT "markets_extended_marketId_key" UNIQUE ("marketId");
ALTER TABLE markets_extended ADD CONSTRAINT "markets_extended_adjTicker_key" UNIQUE ("adjTicker");
ALTER TABLE markets_extended ADD CONSTRAINT "markets_extended_blockchainMarketId_key" UNIQUE ("blockchainMarketId");

-- Step 4: Update trigger
DROP TRIGGER IF EXISTS update_markets_extended_updated_at ON markets_extended;
CREATE TRIGGER update_markets_extended_updated_at
    BEFORE UPDATE ON markets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Step 5: Add comments
COMMENT ON TABLE markets_extended IS 'Markets table - aligned with Prisma schema (camelCase columns)';
COMMENT ON COLUMN markets_extended.id IS 'Primary key - UUID/CUID';
COMMENT ON COLUMN markets_extended."marketId" IS 'Market ID from Adjacent API (unique identifier for external markets)';
COMMENT ON COLUMN markets_extended."blockchainMarketId" IS 'Market ID from blockchain contract (null for non-blockchain markets)';
COMMENT ON COLUMN markets_extended."adjTicker" IS 'Human-readable ticker from Adjacent API';
