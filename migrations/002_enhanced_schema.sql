-- Enhanced Schema Migration for Kizo Server
-- Extends the indexer tables with additional business logic tables

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

-- Yield records table
CREATE TABLE IF NOT EXISTS yield_records (
    id TEXT PRIMARY KEY,
    market_id TEXT NOT NULL,
    protocol_id TEXT NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    apy NUMERIC(10, 6) NOT NULL,
    yield_amount NUMERIC(78, 18) NOT NULL,
    period TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (protocol_id) REFERENCES protocols(id)
);

CREATE INDEX idx_yield_records_market_id ON yield_records(market_id);
CREATE INDEX idx_yield_records_protocol_id ON yield_records(protocol_id);
CREATE INDEX idx_yield_records_period ON yield_records(period);

-- Fee records table
CREATE TABLE IF NOT EXISTS fee_records (
    id TEXT PRIMARY KEY,
    market_id TEXT,
    fee_type TEXT NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    source TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_fee_records_market_id ON fee_records(market_id);
CREATE INDEX idx_fee_records_fee_type ON fee_records(fee_type);

-- Extended markets table (supplements the indexer's markets table)
CREATE TABLE IF NOT EXISTS markets_extended (
    id TEXT PRIMARY KEY,
    market_id BIGINT UNIQUE NOT NULL,
    adj_ticker TEXT UNIQUE,
    platform TEXT NOT NULL DEFAULT 'aptos',
    description TEXT,
    rules TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    probability INT NOT NULL DEFAULT 50,
    volume NUMERIC(78, 18) NOT NULL DEFAULT 0,
    open_interest NUMERIC(78, 18) NOT NULL DEFAULT 0,
    end_date TIMESTAMP NOT NULL,
    resolution_date TIMESTAMP,
    result BOOLEAN,
    link TEXT,
    image_url TEXT,
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
    bet_id BIGINT UNIQUE NOT NULL,
    user_id TEXT NOT NULL,
    odds NUMERIC(10, 4) NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    payout NUMERIC(78, 18),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    FOREIGN KEY (bet_id) REFERENCES bets(bet_id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_bets_extended_bet_id ON bets_extended(bet_id);
CREATE INDEX idx_bets_extended_user_id ON bets_extended(user_id);
CREATE INDEX idx_bets_extended_status ON bets_extended(status);

-- Blockchain events table for sync tracking
CREATE TABLE IF NOT EXISTS blockchain_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    blockchain_id TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT false,
    data TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    UNIQUE(event_type, blockchain_id)
);

CREATE INDEX idx_blockchain_events_type_processed ON blockchain_events(event_type, processed);
CREATE INDEX idx_blockchain_events_block_number ON blockchain_events(block_number);

-- Sync status table
CREATE TABLE IF NOT EXISTS sync_status (
    id TEXT PRIMARY KEY,
    event_type TEXT UNIQUE NOT NULL,
    last_sync_block BIGINT NOT NULL DEFAULT 0,
    last_sync_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Updated timestamp trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply updated_at triggers
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_protocols_updated_at BEFORE UPDATE ON protocols 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_markets_extended_updated_at BEFORE UPDATE ON markets_extended 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_bets_extended_updated_at BEFORE UPDATE ON bets_extended 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_blockchain_events_updated_at BEFORE UPDATE ON blockchain_events 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_sync_status_updated_at BEFORE UPDATE ON sync_status 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();