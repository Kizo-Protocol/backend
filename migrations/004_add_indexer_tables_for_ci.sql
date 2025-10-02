-- Add basic indexer tables that backend depends on
-- These are minimal versions for CI testing

-- Basic bets table (normally created by indexer)
CREATE TABLE IF NOT EXISTS bets (
    bet_id BIGINT PRIMARY KEY,
    user_addr TEXT NOT NULL,
    market_id BIGINT NOT NULL,
    position TEXT NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    odds NUMERIC(10, 4) NOT NULL,
    transaction_version BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_bets_market_id ON bets(market_id);
CREATE INDEX IF NOT EXISTS idx_bets_user_addr ON bets(user_addr);
CREATE INDEX IF NOT EXISTS idx_bets_inserted_at ON bets(inserted_at);

-- Basic markets table (normally created by indexer)
CREATE TABLE IF NOT EXISTS markets (
    market_id BIGINT PRIMARY KEY,
    question TEXT NOT NULL,
    end_time TIMESTAMP NOT NULL,
    creator_addr TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    transaction_version BIGINT NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_markets_status ON markets(status);
CREATE INDEX IF NOT EXISTS idx_markets_end_time ON markets(end_time);
