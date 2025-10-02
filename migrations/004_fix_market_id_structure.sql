-- Migration to fix market ID structure
-- Following the Prisma schema pattern: id (PK), marketId (Adjacent API), blockchainMarketId (blockchain)

-- Rename current market_id to blockchain_market_id
ALTER TABLE markets_extended RENAME COLUMN market_id TO blockchain_market_id;

-- Make blockchain_market_id nullable since Adjacent API markets don't have blockchain IDs
ALTER TABLE markets_extended ALTER COLUMN blockchain_market_id DROP NOT NULL;

-- Add adj_market_id column for Adjacent API market IDs
ALTER TABLE markets_extended ADD COLUMN IF NOT EXISTS adj_market_id TEXT;

-- Update existing rows: use adj_ticker as adj_market_id for now
UPDATE markets_extended SET adj_market_id = adj_ticker WHERE adj_market_id IS NULL;

-- Add unique constraint on adj_market_id
ALTER TABLE markets_extended ADD CONSTRAINT markets_extended_adj_market_id_key UNIQUE (adj_market_id);

-- Drop the old index on market_id (now blockchain_market_id)
DROP INDEX IF EXISTS idx_markets_extended_market_id;

-- Create new indexes
CREATE INDEX IF NOT EXISTS idx_markets_extended_blockchain_market_id ON markets_extended(blockchain_market_id) WHERE blockchain_market_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_markets_extended_adj_market_id ON markets_extended(adj_market_id);

-- Add comments for clarity
COMMENT ON COLUMN markets_extended.id IS 'Primary key - UUID/CUID';
COMMENT ON COLUMN markets_extended.adj_market_id IS 'Market ID from Adjacent API (unique identifier for external markets)';
COMMENT ON COLUMN markets_extended.blockchain_market_id IS 'Market ID from blockchain contract (null for non-blockchain markets)';
COMMENT ON COLUMN markets_extended.adj_ticker IS 'Human-readable ticker from Adjacent API';