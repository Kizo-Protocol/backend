-- Simplified Enhanced Schema Migration for Kizo Server
-- Basic tables without complex foreign key constraints

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    address TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    username TEXT UNIQUE,
    avatar_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_address ON users(address);

-- Protocols table for yield farming
CREATE TABLE IF NOT EXISTS protocols (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    base_apy NUMERIC(10, 6) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    icon_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Extended markets table (supplements the indexer's markets table)
CREATE TABLE IF NOT EXISTS markets_extended (
    id TEXT PRIMARY KEY,
    market_id TEXT UNIQUE NOT NULL,
    adj_ticker TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    end_date TIMESTAMP,
    winner TEXT,
    total_pool_size NUMERIC(78, 18) NOT NULL DEFAULT 0,
    yes_pool_size NUMERIC(78, 18) NOT NULL DEFAULT 0,
    no_pool_size NUMERIC(78, 18) NOT NULL DEFAULT 0,
    count_yes INT NOT NULL DEFAULT 0,
    count_no INT NOT NULL DEFAULT 0,
    current_yield NUMERIC(78, 18) NOT NULL DEFAULT 0,
    total_yield_earned_decimal NUMERIC(78, 18) NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_markets_extended_market_id ON markets_extended(market_id);
CREATE INDEX idx_markets_extended_adj_ticker ON markets_extended(adj_ticker);
CREATE INDEX idx_markets_extended_status ON markets_extended(status);
CREATE INDEX idx_markets_extended_end_date ON markets_extended(end_date);

-- Extended bets table (supplements the indexer's bets table)
CREATE TABLE IF NOT EXISTS bets_extended (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    market_id TEXT NOT NULL,
    blockchain_bet_id TEXT UNIQUE NOT NULL,
    position TEXT NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    odds NUMERIC(10, 4) NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    payout NUMERIC(78, 18),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_bets_extended_bet_id ON bets_extended(blockchain_bet_id);
CREATE INDEX idx_bets_extended_user_id ON bets_extended(user_id);
CREATE INDEX idx_bets_extended_status ON bets_extended(status);

-- Event processing log table
CREATE TABLE IF NOT EXISTS event_processing_log (
    id SERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    transaction_version BIGINT,
    processing_status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    processing_duration_ms INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_event_processing_log_event_type ON event_processing_log(event_type);
CREATE INDEX idx_event_processing_log_status ON event_processing_log(processing_status);
CREATE INDEX idx_event_processing_log_created_at ON event_processing_log(created_at);
