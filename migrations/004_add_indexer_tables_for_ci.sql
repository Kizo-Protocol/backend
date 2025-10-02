-- Add basic indexer tables that backend depends on
-- Schema matches the actual indexer database structure

-- Basic bets table (normally created by indexer)
CREATE TABLE IF NOT EXISTS bets (
    bet_id BIGINT PRIMARY KEY,
    market_id BIGINT NOT NULL,
    user_addr VARCHAR(66) NOT NULL,
    position BOOLEAN NOT NULL,
    amount BIGINT NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    claimed BOOLEAN DEFAULT FALSE,
    winning_amount BIGINT DEFAULT 0,
    yield_share BIGINT DEFAULT 0,
    claim_transaction_version BIGINT
);

CREATE INDEX IF NOT EXISTS idx_bets_market_id ON bets(market_id);
CREATE INDEX IF NOT EXISTS idx_bets_user_addr ON bets(user_addr);
CREATE INDEX IF NOT EXISTS idx_bets_inserted_at ON bets(inserted_at);
CREATE INDEX IF NOT EXISTS idx_bets_claimed ON bets(claimed);

-- Basic markets table (normally created by indexer)
CREATE TABLE IF NOT EXISTS markets (
    market_id BIGINT PRIMARY KEY,
    question TEXT NOT NULL,
    end_time BIGINT NOT NULL,
    yield_protocol_addr VARCHAR(66) NOT NULL,
    transaction_version BIGINT NOT NULL,
    transaction_block_height BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    resolved BOOLEAN DEFAULT FALSE,
    outcome BOOLEAN,
    total_yield_earned BIGINT DEFAULT 0,
    resolution_transaction_version BIGINT
);

CREATE INDEX IF NOT EXISTS idx_markets_resolved ON markets(resolved);
CREATE INDEX IF NOT EXISTS idx_markets_end_time ON markets(end_time);
CREATE INDEX IF NOT EXISTS idx_markets_inserted_at ON markets(inserted_at);
