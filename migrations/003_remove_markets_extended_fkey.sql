-- Migration to allow seeding markets from Adjacent API
-- These markets don't have corresponding blockchain entries

-- Remove foreign key constraint on market_id
-- This allows markets_extended to contain both blockchain-synced markets
-- and markets seeded from external APIs like Adjacent
ALTER TABLE markets_extended DROP CONSTRAINT IF EXISTS markets_extended_market_id_fkey;

-- Remove unique constraint on market_id
-- This allows for generated market_ids from external sources
-- adj_ticker is still unique and serves as the primary identifier for seeded markets
ALTER TABLE markets_extended DROP CONSTRAINT IF EXISTS markets_extended_market_id_key;

-- Note: Blockchain-synced markets will still reference real market_id values from the markets table
-- Seeded markets will have generated market_id values that don't reference blockchain data