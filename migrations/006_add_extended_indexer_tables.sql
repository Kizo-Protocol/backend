-- Add extended indexer tables from the actual database schema
-- These support additional functionality like resolutions, events, and claims

-- Bets extended table with additional metadata (matches actual schema)
CREATE TABLE IF NOT EXISTS bets_extended (
    id TEXT PRIMARY KEY,
    "blockchainBetId" BIGINT NOT NULL UNIQUE,
    "userId" TEXT NOT NULL,
    "marketId" TEXT,
    position BOOLEAN,
    amount NUMERIC(78, 18),
    odds NUMERIC(10, 4) NOT NULL,
    status TEXT DEFAULT 'active' NOT NULL,
    payout NUMERIC(78, 18),
    "createdAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updatedAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bets_extended_blockchainBetId ON bets_extended("blockchainBetId");
CREATE INDEX IF NOT EXISTS idx_bets_extended_marketId ON bets_extended("marketId");
CREATE INDEX IF NOT EXISTS idx_bets_extended_status ON bets_extended(status);
CREATE INDEX IF NOT EXISTS idx_bets_extended_userId ON bets_extended("userId");

-- Market resolutions table (without total_yes_pool, total_no_pool)
CREATE TABLE IF NOT EXISTS market_resolutions (
    market_id BIGINT PRIMARY KEY,
    outcome BOOLEAN NOT NULL,
    total_yield_earned BIGINT NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_market_resolutions_transaction_version ON market_resolutions(transaction_version);

-- Blockchain events table (matches actual camelCase schema)
CREATE TABLE IF NOT EXISTS blockchain_events (
    id TEXT PRIMARY KEY,
    "eventType" TEXT NOT NULL,
    "blockchainId" TEXT NOT NULL,
    "blockNumber" BIGINT NOT NULL,
    "blockTimestamp" BIGINT NOT NULL,
    "transactionHash" TEXT NOT NULL,
    processed BOOLEAN DEFAULT FALSE NOT NULL,
    data TEXT,
    "createdAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updatedAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE("eventType", "blockchainId")
);

CREATE INDEX IF NOT EXISTS idx_blockchain_events_blockNumber ON blockchain_events("blockNumber");
CREATE INDEX IF NOT EXISTS idx_blockchain_events_eventType_processed ON blockchain_events("eventType", processed);

-- Winnings claims table (without market_id)
CREATE TABLE IF NOT EXISTS winnings_claims (
    claim_id BIGSERIAL PRIMARY KEY,
    bet_id BIGINT NOT NULL,
    user_addr VARCHAR(66) NOT NULL,
    winning_amount BIGINT NOT NULL,
    yield_share BIGINT NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_winnings_claims_bet_id ON winnings_claims(bet_id);
CREATE INDEX IF NOT EXISTS idx_winnings_claims_transaction_version ON winnings_claims(transaction_version);
CREATE INDEX IF NOT EXISTS idx_winnings_claims_user_addr ON winnings_claims(user_addr);

-- Yield deposits table (with 'amount' instead of deposit_amount/yield_earned)
CREATE TABLE IF NOT EXISTS yield_deposits (
    deposit_id BIGSERIAL PRIMARY KEY,
    market_id BIGINT NOT NULL,
    amount BIGINT NOT NULL,
    protocol_addr VARCHAR(66) NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_yield_deposits_market_id ON yield_deposits(market_id);
CREATE INDEX IF NOT EXISTS idx_yield_deposits_transaction_version ON yield_deposits(transaction_version);

-- Sync status table (matches actual camelCase schema)
CREATE TABLE IF NOT EXISTS sync_status (
    id TEXT PRIMARY KEY,
    "eventType" TEXT NOT NULL UNIQUE,
    "lastSyncBlock" BIGINT DEFAULT 0 NOT NULL,
    "lastSyncTime" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "isActive" BOOLEAN DEFAULT TRUE NOT NULL,
    "createdAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updatedAt" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
