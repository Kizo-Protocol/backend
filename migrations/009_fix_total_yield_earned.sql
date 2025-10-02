-- Migration: Fix totalYieldEarned values
-- totalYieldEarned should be incremental and only updated via yield_deposit_event
-- It should NOT equal totalPoolSize

-- Reset totalYieldEarned to 0 for all active markets
-- (It will be properly incremented as yield deposit events occur)
UPDATE markets_extended
SET "totalYieldEarned" = 0,
    "updatedAt" = NOW()
WHERE status = 'active';

-- For resolved markets, keep totalYieldEarned as is
-- (These represent the final yield that was earned when market was resolved)

-- Add a comment to the column for documentation
COMMENT ON COLUMN markets_extended."totalYieldEarned" IS 
'Incremental total yield earned from yield farming protocols. 
Updated only via yield_deposit_event from blockchain. 
Should start at 0 and accumulate over time. 
For active markets, use currentYield for real-time calculated yield.';

COMMENT ON COLUMN markets_extended."currentYield" IS 
'Calculated current yield based on pool size, APY, and time elapsed. 
This is computed in real-time and represents projected yield so far.';
