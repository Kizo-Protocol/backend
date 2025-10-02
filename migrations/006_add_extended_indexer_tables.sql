-- Add extended indexer tables from the actual database schema
-- These support additional functionality like resolutions, events, and claims

-- Bets extended table with additional metadata
CREATE TABLE IF NOT EXISTS bets_extended (
    id TEXT PRIMARY KEY,
    "blockchainBetId" BIGINT,
    "betId" TEXT,
    "marketId" TEXT,
    "userId" TEXT,
    platform TEXT DEFAULT 'aptos' NOT NULL,
    "userAddr" TEXT NOT NULL,
    position BOOLEAN NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    odds NUMERIC(10, 4),
    "potentialWinnings" NUMERIC(78, 18),
    status TEXT DEFAULT 'active' NOT NULL,
    claimed BOOLEAN DEFAULT FALSE,
    "claimAmount" NUMERIC(78, 18),
    "placedAt" TIMESTAMP NOT NULL,
    "settledAt" TIMESTAMP,
    "createdAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updatedAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bets_extended_blockchain_bet_id ON bets_extended("blockchainBetId");
CREATE INDEX IF NOT EXISTS idx_bets_extended_market_id ON bets_extended("marketId");
CREATE INDEX IF NOT EXISTS idx_bets_extended_user_addr ON bets_extended("userAddr");
CREATE INDEX IF NOT EXISTS idx_bets_extended_status ON bets_extended(status);

-- Market resolutions table
CREATE TABLE IF NOT EXISTS market_resolutions (
    market_id BIGINT PRIMARY KEY,
    outcome BOOLEAN NOT NULL,
    total_yes_pool BIGINT NOT NULL,
    total_no_pool BIGINT NOT NULL,
    total_yield_earned BIGINT NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_market_resolutions_inserted_at ON market_resolutions(inserted_at);

-- Blockchain events table for event tracking
CREATE TABLE IF NOT EXISTS blockchain_events (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    market_id BIGINT,
    user_addr VARCHAR(66),
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    event_data JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_blockchain_events_type ON blockchain_events(event_type);
CREATE INDEX IF NOT EXISTS idx_blockchain_events_market_id ON blockchain_events(market_id);
CREATE INDEX IF NOT EXISTS idx_blockchain_events_transaction_version ON blockchain_events(transaction_version);
CREATE INDEX IF NOT EXISTS idx_blockchain_events_created_at ON blockchain_events(created_at);

-- Winnings claims table
CREATE TABLE IF NOT EXISTS winnings_claims (
    claim_id SERIAL PRIMARY KEY,
    bet_id BIGINT NOT NULL,
    market_id BIGINT NOT NULL,
    user_addr VARCHAR(66) NOT NULL,
    winning_amount BIGINT NOT NULL,
    yield_share BIGINT NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_winnings_claims_bet_id ON winnings_claims(bet_id);
CREATE INDEX IF NOT EXISTS idx_winnings_claims_market_id ON winnings_claims(market_id);
CREATE INDEX IF NOT EXISTS idx_winnings_claims_user_addr ON winnings_claims(user_addr);

-- Yield deposits table
CREATE TABLE IF NOT EXISTS yield_deposits (
    deposit_id SERIAL PRIMARY KEY,
    market_id BIGINT NOT NULL,
    protocol_addr VARCHAR(66) NOT NULL,
    deposit_amount BIGINT NOT NULL,
    yield_earned BIGINT NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_yield_deposits_market_id ON yield_deposits(market_id);
CREATE INDEX IF NOT EXISTS idx_yield_deposits_inserted_at ON yield_deposits(inserted_at);

-- Sync status table for tracking indexer progress
CREATE TABLE IF NOT EXISTS sync_status (
    id SERIAL PRIMARY KEY,
    indexer_name TEXT NOT NULL UNIQUE,
    last_transaction_version BIGINT NOT NULL DEFAULT 0,
    last_success_transaction_version BIGINT NOT NULL DEFAULT 0,
    last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
    is_synced BOOLEAN DEFAULT FALSE
);
