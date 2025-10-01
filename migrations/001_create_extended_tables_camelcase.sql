-- Enhanced Schema Migration for Kizo Server with camelCase columns
-- Extends the indexer tables with additional business logic tables

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    address TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    username TEXT UNIQUE,
    "avatarUrl" TEXT,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_address ON users(address);

-- Protocols table for yield farming
CREATE TABLE IF NOT EXISTS protocols (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    "displayName" TEXT NOT NULL,
    "baseApy" NUMERIC(10, 6) NOT NULL DEFAULT 0,
    "isActive" BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    "iconUrl" TEXT,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Yield records table
CREATE TABLE IF NOT EXISTS yield_records (
    id TEXT PRIMARY KEY,
    "marketId" TEXT NOT NULL,
    "protocolId" TEXT NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    apy NUMERIC(10, 6) NOT NULL,
    yield NUMERIC(78, 18) NOT NULL,
    period TIMESTAMP NOT NULL,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY ("protocolId") REFERENCES protocols(id)
);

CREATE INDEX "idx_yield_records_marketId" ON yield_records("marketId");
CREATE INDEX "idx_yield_records_protocolId" ON yield_records("protocolId");
CREATE INDEX idx_yield_records_period ON yield_records(period);

-- Fee records table
CREATE TABLE IF NOT EXISTS fee_records (
    id TEXT PRIMARY KEY,
    "marketId" TEXT,
    "feeType" TEXT NOT NULL,
    amount NUMERIC(78, 18) NOT NULL,
    source TEXT NOT NULL,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX "idx_fee_records_marketId" ON fee_records("marketId");
CREATE INDEX "idx_fee_records_feeType" ON fee_records("feeType");

-- Extended markets table (supplements the indexer's markets table)
CREATE TABLE IF NOT EXISTS markets_extended (
    id TEXT PRIMARY KEY,
    "blockchainMarketId" BIGINT UNIQUE,
    "marketId" TEXT UNIQUE,
    "adjTicker" TEXT UNIQUE,
    platform TEXT NOT NULL DEFAULT 'aptos',
    question TEXT,
    description TEXT,
    rules TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    probability INT NOT NULL DEFAULT 50,
    volume NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "openInterest" NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "endDate" TIMESTAMP NOT NULL,
    "resolutionDate" TIMESTAMP,
    result BOOLEAN,
    link TEXT,
    "imageUrl" TEXT,
    "totalPoolSize" NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "yesPoolSize" NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "noPoolSize" NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "countYes" INT NOT NULL DEFAULT 0,
    "countNo" INT NOT NULL DEFAULT 0,
    "currentYield" NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "totalYieldEarned" NUMERIC(78, 18) NOT NULL DEFAULT 0,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX "idx_markets_extended_blockchainMarketId" ON markets_extended("blockchainMarketId") WHERE "blockchainMarketId" IS NOT NULL;
CREATE INDEX "idx_markets_extended_marketId" ON markets_extended("marketId");
CREATE INDEX "idx_markets_extended_adjTicker" ON markets_extended("adjTicker");
CREATE INDEX idx_markets_extended_status ON markets_extended(status);
CREATE INDEX "idx_markets_extended_endDate" ON markets_extended("endDate");

-- Extended bets table (supplements the indexer's bets table)
CREATE TABLE IF NOT EXISTS bets_extended (
    id TEXT PRIMARY KEY,
    "blockchainBetId" BIGINT UNIQUE NOT NULL,
    "userId" TEXT NOT NULL,
    "marketId" TEXT,
    position BOOLEAN,
    amount NUMERIC(78, 18),
    odds NUMERIC(10, 4) NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    payout NUMERIC(78, 18),
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY ("userId") REFERENCES users(id)
);

CREATE INDEX "idx_bets_extended_blockchainBetId" ON bets_extended("blockchainBetId");
CREATE INDEX "idx_bets_extended_userId" ON bets_extended("userId");
CREATE INDEX "idx_bets_extended_marketId" ON bets_extended("marketId");
CREATE INDEX idx_bets_extended_status ON bets_extended(status);

-- Blockchain events table for sync tracking
CREATE TABLE IF NOT EXISTS blockchain_events (
    id TEXT PRIMARY KEY,
    "eventType" TEXT NOT NULL,
    "blockchainId" TEXT NOT NULL,
    "blockNumber" BIGINT NOT NULL,
    "blockTimestamp" BIGINT NOT NULL,
    "transactionHash" TEXT NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT false,
    data TEXT,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE("eventType", "blockchainId")
);

CREATE INDEX "idx_blockchain_events_type_processed" ON blockchain_events("eventType", processed);
CREATE INDEX "idx_blockchain_events_blockNumber" ON blockchain_events("blockNumber");

-- Sync status table
CREATE TABLE IF NOT EXISTS sync_status (
    id TEXT PRIMARY KEY,
    "eventType" TEXT UNIQUE NOT NULL,
    "lastSyncBlock" BIGINT NOT NULL DEFAULT 0,
    "lastSyncTime" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "isActive" BOOLEAN NOT NULL DEFAULT true,
    "createdAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Updated timestamp trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW."updatedAt" = CURRENT_TIMESTAMP;
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