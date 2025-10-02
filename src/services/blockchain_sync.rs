use anyhow::Result;
use bigdecimal::ToPrimitive;
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug)]
pub struct SyncResult {
    pub event_type: String,
    pub processed: i64,
    pub errors: i64,
    pub new_events: i64,
    pub skipped: i64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SyncSummary {
    pub total_processed: i64,
    pub total_errors: i64,
    pub results: Vec<SyncResult>,
    pub duration_ms: u128,
}

pub struct BlockchainSyncService {
    pool: PgPool,
}

impl BlockchainSyncService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn sync_markets(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        let mut result = SyncResult {
            event_type: "MarketSync".to_string(),
            processed: 0,
            errors: 0,
            new_events: 0,
            skipped: 0,
        };

        info!("Starting market sync from indexer tables");

        let markets = sqlx::query!(
            r#"
            SELECT m.market_id, m.question, m.end_time, 
                   m.transaction_version, m.resolved as status
            FROM markets m
            LEFT JOIN markets_extended me ON m.market_id = me."blockchainMarketId"
            WHERE me.id IS NULL
            ORDER BY m.transaction_version DESC
            LIMIT 100
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        info!("Found {} markets to sync", markets.len());

        for market in markets {
            result.processed += 1;

            let market_id_val = market.market_id;
            match self
                .create_extended_market(market.market_id, chrono::Utc::now().timestamp())
                .await
            {
                Ok(_) => {
                    result.new_events += 1;
                    info!("Created extended record for market {}", market_id_val);
                }
                Err(e) => {
                    result.errors += 1;
                    error!("Failed to create extended market {}: {}", market_id_val, e);
                }
            }
        }

        let duration = start.elapsed();
        info!(
            "Market sync completed in {:?}: {} processed, {} new, {} errors",
            duration, result.processed, result.new_events, result.errors
        );

        Ok(result)
    }

    async fn create_extended_market(&self, market_id: i64, end_time: i64) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let end_date = chrono::DateTime::from_timestamp(end_time, 0)
            .map(|dt| dt.naive_utc())
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());

        sqlx::query!(
            r#"
            INSERT INTO markets_extended (
                id, "blockchainMarketId", platform, status, probability,
                "endDate", "createdAt", "updatedAt"
            )
            VALUES ($1, $2, 'aptos', 'active', 50, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT ("blockchainMarketId") DO NOTHING
            "#,
            id,
            market_id,
            end_date
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn sync_bets(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        let mut result = SyncResult {
            event_type: "BetSync".to_string(),
            processed: 0,
            errors: 0,
            new_events: 0,
            skipped: 0,
        };

        info!("Starting bet sync from indexer tables");

        let bets = sqlx::query!(
            r#"
            SELECT b.bet_id, b.user_addr, b.amount
            FROM bets b
            LEFT JOIN bets_extended be ON b.bet_id = be."blockchainBetId"
            WHERE be.id IS NULL
            ORDER BY b.transaction_version DESC
            LIMIT 500
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        info!("Found {} bets to sync", bets.len());

        for bet in bets {
            result.processed += 1;

            let user_id = match self.get_or_create_user(&bet.user_addr).await {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to get/create user {}: {}", bet.user_addr, e);
                    result.errors += 1;
                    continue;
                }
            };

            // Convert BigDecimal to i64 for amount
            let amount_i64 = bet.amount.to_i64().unwrap_or(0);

            match self
                .create_extended_bet(bet.bet_id, &user_id, amount_i64)
                .await
            {
                Ok(_) => {
                    result.new_events += 1;
                }
                Err(e) => {
                    error!("Failed to create extended bet {}: {}", bet.bet_id, e);
                    result.errors += 1;
                }
            }
        }

        let duration = start.elapsed();
        info!(
            "Bet sync completed in {:?}: {} processed, {} new, {} errors",
            duration, result.processed, result.new_events, result.errors
        );

        Ok(result)
    }

    async fn get_or_create_user(&self, address: &str) -> Result<String> {
        let normalized_addr = address.to_lowercase();

        let existing = sqlx::query!("SELECT id FROM users WHERE address = $1", normalized_addr)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(user) = existing {
            return Ok(user.id);
        }

        let id = Uuid::new_v4().to_string();

        match sqlx::query!(
            r#"
            INSERT INTO users (id, address, "createdAt", "updatedAt")
            VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT (address) DO NOTHING
            "#,
            id,
            normalized_addr
        )
        .execute(&self.pool)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    info!("Created new user: {}", normalized_addr);
                    Ok(id)
                } else {
                    let user =
                        sqlx::query!("SELECT id FROM users WHERE address = $1", normalized_addr)
                            .fetch_one(&self.pool)
                            .await?;
                    Ok(user.id)
                }
            }
            Err(e) => {
                warn!(
                    "Failed to insert user {}: {}, attempting to fetch",
                    normalized_addr, e
                );
                let user = sqlx::query!("SELECT id FROM users WHERE address = $1", normalized_addr)
                    .fetch_one(&self.pool)
                    .await?;
                Ok(user.id)
            }
        }
    }

    async fn create_extended_bet(&self, bet_id: i64, user_id: &str, amount: i64) -> Result<()> {
        let bet = sqlx::query!(
            r#"
            SELECT bet_id, market_id, user_addr, position, amount
            FROM bets
            WHERE bet_id = $1
            "#,
            bet_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let bet = match bet {
            Some(b) => b,
            None => {
                warn!("Bet {} not found in indexer, skipping", bet_id);
                return Ok(());
            }
        };

        let market = sqlx::query!(
            r#"SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1"#,
            bet.market_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let market_uuid = match market {
            Some(m) => m.id,
            None => {
                warn!(
                    "Market {} not found for bet {}, skipping",
                    bet.market_id, bet_id
                );
                return Ok(());
            }
        };

        let id = Uuid::new_v4().to_string();
        let odds = sqlx::types::BigDecimal::from(1);
        let amount_decimal = sqlx::types::BigDecimal::from(amount);

        // Convert position string to boolean
        let position_bool = bet.position;

        sqlx::query!(
            r#"
            INSERT INTO bets_extended (
                id, "blockchainBetId", "userId", "marketId", position, amount, odds, status, "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT ("blockchainBetId") DO NOTHING
            "#,
            id,
            bet_id,
            user_id,
            market_uuid,
            position_bool,
            amount_decimal,
            odds
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_market_stats(&self) -> Result<()> {
        info!("Updating market statistics");

        let updated_rows = sqlx::query!(
            r#"
            UPDATE markets_extended me
            SET
                "yesPoolSize" = COALESCE(subq.yes_pool, 0),
                "noPoolSize" = COALESCE(subq.no_pool, 0),
                "totalPoolSize" = COALESCE(subq.total_pool, 0),
                "countYes" = COALESCE(subq.yes_count, 0),
                "countNo" = COALESCE(subq.no_count, 0),
                volume = COALESCE(subq.total_pool, 0),
                probability = CASE
                    WHEN COALESCE(subq.total_pool, 0) > 0 THEN
                        ROUND((COALESCE(subq.yes_pool, 0) / subq.total_pool * 100)::numeric)::int
                    ELSE 50
                END,
                "updatedAt" = CURRENT_TIMESTAMP
            FROM (
                SELECT
                    me2."blockchainMarketId" as market_id,
                    SUM(CASE WHEN be.position = true THEN be.amount ELSE 0 END)::numeric as yes_pool,
                    SUM(CASE WHEN be.position = false THEN be.amount ELSE 0 END)::numeric as no_pool,
                    SUM(be.amount)::numeric as total_pool,
                    COUNT(CASE WHEN be.position = true THEN 1 END)::int as yes_count,
                    COUNT(CASE WHEN be.position = false THEN 1 END)::int as no_count
                FROM markets_extended me2
                LEFT JOIN bets_extended be ON be."marketId" = me2.id
                WHERE be.status = 'active'
                GROUP BY me2."blockchainMarketId"
            ) subq
            WHERE me."blockchainMarketId" = subq.market_id
            "#
        )
        .execute(&self.pool)
        .await?;

        info!(
            "Market statistics updated for {} markets",
            updated_rows.rows_affected()
        );
        Ok(())
    }

    pub async fn update_market_stats_for_market(&self, blockchain_market_id: &str) -> Result<()> {
        info!(
            "Updating statistics for blockchain market_id: {}",
            blockchain_market_id
        );

        let market_id_i64: i64 = blockchain_market_id.parse().unwrap_or_else(|_| {
            warn!(
                "Failed to parse market_id {}, trying as UUID",
                blockchain_market_id
            );
            -1
        });

        let updated_rows = sqlx::query!(
            r#"
            UPDATE markets_extended me
            SET
                "yesPoolSize" = COALESCE(subq.yes_pool, 0),
                "noPoolSize" = COALESCE(subq.no_pool, 0),
                "totalPoolSize" = COALESCE(subq.total_pool, 0),
                "countYes" = COALESCE(subq.yes_count, 0),
                "countNo" = COALESCE(subq.no_count, 0),
                volume = COALESCE(subq.total_pool, 0),
                probability = CASE
                    WHEN COALESCE(subq.total_pool, 0) > 0 THEN
                        ROUND((COALESCE(subq.yes_pool, 0) / subq.total_pool * 100)::numeric)::int
                    ELSE 50
                END,
                "updatedAt" = CURRENT_TIMESTAMP
            FROM (
                SELECT
                    me2."blockchainMarketId",
                    SUM(CASE WHEN be.position = true THEN be.amount ELSE 0 END)::numeric as yes_pool,
                    SUM(CASE WHEN be.position = false THEN be.amount ELSE 0 END)::numeric as no_pool,
                    SUM(be.amount)::numeric as total_pool,
                    COUNT(CASE WHEN be.position = true THEN 1 END)::int as yes_count,
                    COUNT(CASE WHEN be.position = false THEN 1 END)::int as no_count
                FROM markets_extended me2
                LEFT JOIN bets_extended be ON be."marketId" = me2.id AND be.status = 'active'
                WHERE me2."blockchainMarketId" = $1
                GROUP BY me2."blockchainMarketId"
            ) subq
            WHERE me."blockchainMarketId" = subq."blockchainMarketId"
            "#,
            market_id_i64
        )
        .execute(&self.pool)
        .await?;

        info!(
            "Market {} statistics updated ({} rows affected)",
            blockchain_market_id,
            updated_rows.rows_affected()
        );
        Ok(())
    }

    pub async fn run_full_sync(&self) -> Result<SyncSummary> {
        let start = std::time::Instant::now();
        info!("Starting full blockchain sync");

        let mut results = Vec::new();
        let mut total_processed = 0;
        let mut total_errors = 0;

        match self.sync_markets().await {
            Ok(result) => {
                total_processed += result.processed;
                total_errors += result.errors;
                results.push(result);
            }
            Err(e) => {
                error!("Market sync failed: {}", e);
            }
        }

        match self.sync_bets().await {
            Ok(result) => {
                total_processed += result.processed;
                total_errors += result.errors;
                results.push(result);
            }
            Err(e) => {
                error!("Bet sync failed: {}", e);
            }
        }

        if let Err(e) = self.update_market_stats().await {
            error!("Failed to update market stats: {}", e);
        }

        let duration_ms = start.elapsed().as_millis();
        info!(
            "Full sync completed in {}ms: {} total processed, {} errors",
            duration_ms, total_processed, total_errors
        );

        Ok(SyncSummary {
            total_processed,
            total_errors,
            results,
            duration_ms,
        })
    }
}
