use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use super::blockchain_sync::BlockchainSyncService;

#[derive(Debug, Serialize, Deserialize)]
pub struct RealtimeSyncConfig {
    pub bet_sync_interval_ms: u64,
    pub market_sync_interval_ms: u64,
    pub enable_immediate_sync: bool,
    pub max_retries: usize,
}

impl Default for RealtimeSyncConfig {
    fn default() -> Self {
        Self {
            bet_sync_interval_ms: 1000,
            market_sync_interval_ms: 5000,
            enable_immediate_sync: true,
            max_retries: 3,
        }
    }
}

pub struct RealtimeSyncService {
    pool: PgPool,
    config: RealtimeSyncConfig,
    blockchain_sync: BlockchainSyncService,
}

impl RealtimeSyncService {
    pub fn new(pool: PgPool) -> Self {
        let config = RealtimeSyncConfig::default();
        let blockchain_sync = BlockchainSyncService::new(pool.clone());

        Self {
            pool,
            config,
            blockchain_sync,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_config(pool: PgPool, config: RealtimeSyncConfig) -> Self {
        let blockchain_sync = BlockchainSyncService::new(pool.clone());

        Self {
            pool,
            config,
            blockchain_sync,
        }
    }

    #[allow(dead_code)]
    pub async fn start_realtime_sync(self) {
        info!("Starting real-time synchronization service");

        let pool = self.pool.clone();
        let config = self.config.clone();
        let _blockchain_sync = self.blockchain_sync;

        let bet_sync_pool = pool.clone();
        let bet_sync_config = config.clone();
        tokio::spawn(async move {
            let mut interval =
                time::interval(Duration::from_millis(bet_sync_config.bet_sync_interval_ms));
            let sync_service = BlockchainSyncService::new(bet_sync_pool);

            loop {
                interval.tick().await;

                match sync_service.sync_bets().await {
                    Ok(result) => {
                        if result.new_events > 0 {
                            info!(
                                "Real-time bet sync: {} new bets processed",
                                result.new_events
                            );

                            if let Err(e) = sync_service.update_market_stats().await {
                                error!("Failed to update market stats in real-time: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Real-time bet sync failed: {}", e);
                    }
                }
            }
        });

        let market_sync_pool = pool.clone();
        let market_sync_config = config.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(
                market_sync_config.market_sync_interval_ms,
            ));
            let sync_service = BlockchainSyncService::new(market_sync_pool);

            loop {
                interval.tick().await;

                match sync_service.sync_markets().await {
                    Ok(result) => {
                        if result.new_events > 0 {
                            info!(
                                "Real-time market sync: {} new markets processed",
                                result.new_events
                            );
                        }
                    }
                    Err(e) => {
                        error!("Real-time market sync failed: {}", e);
                    }
                }
            }
        });

        info!("Real-time synchronization service started successfully");
    }

    pub async fn sync_market_immediately(&self, market_id: &str) -> Result<()> {
        info!("Triggering immediate sync for market: {}", market_id);

        let bet_result = self.blockchain_sync.sync_bets().await?;
        if bet_result.new_events > 0 {
            info!("Immediate sync found {} new bets", bet_result.new_events);
        }

        self.blockchain_sync
            .update_market_stats_for_market(market_id)
            .await?;

        if let Err(e) = self.recalculate_market_yield(market_id).await {
            warn!(
                "Failed to recalculate yield for market {}: {}",
                market_id, e
            );
        }

        info!("Immediate sync completed for market: {}", market_id);
        Ok(())
    }

    async fn recalculate_market_yield(&self, market_id: &str) -> Result<()> {
        let market = sqlx::query!(
            r#"
            SELECT id, "blockchainMarketId", "endDate", "totalPoolSize"
            FROM markets_extended
            WHERE id = $1 OR "adjTicker" = $1 OR "blockchainMarketId"::text = $1
            LIMIT 1
            "#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("Market not found: {}", market_id))?;

        let yield_data = super::yield_calculator::calculate_market_yield_data(
            &self.pool,
            &market.totalPoolSize,
            &market.totalPoolSize,
            &market.endDate,
        )
        .await;

        if let Ok(yd) = yield_data {
            let daily_yield_decimal = sqlx::types::BigDecimal::try_from(yd.daily_yield)
                .unwrap_or_else(|_| sqlx::types::BigDecimal::from(0));

            sqlx::query!(
                r#"
                UPDATE markets_extended
                SET "currentYield" = $1, "updatedAt" = NOW()
                WHERE id = $2
                "#,
                daily_yield_decimal,
                market.id
            )
            .execute(&self.pool)
            .await?;

            info!(
                "Yield recalculated for market: {} (daily yield: {})",
                market_id, yd.daily_yield
            );
        } else {
            warn!("Failed to calculate yield data for market: {}", market_id);
        }

        Ok(())
    }

    pub async fn get_sync_stats(&self) -> Result<RealtimeSyncStats> {
        let last_bet_sync = sqlx::query_scalar!("SELECT MAX(\"updatedAt\") FROM bets_extended")
            .fetch_optional(&self.pool)
            .await?
            .flatten();

        let last_market_sync =
            sqlx::query_scalar!("SELECT MAX(\"updatedAt\") FROM markets_extended")
                .fetch_optional(&self.pool)
                .await?
                .flatten();

        let pending_bets = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM bets b
            LEFT JOIN bets_extended be ON b.bet_id = be."blockchainBetId"
            WHERE be.id IS NULL
            "#
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(RealtimeSyncStats {
            last_bet_sync,
            last_market_sync,
            pending_bets,
            config: self.config.clone(),
            is_active: true,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct RealtimeSyncStats {
    pub last_bet_sync: Option<chrono::NaiveDateTime>,
    pub last_market_sync: Option<chrono::NaiveDateTime>,
    pub pending_bets: i64,
    pub config: RealtimeSyncConfig,
    pub is_active: bool,
}

impl Clone for RealtimeSyncConfig {
    fn clone(&self) -> Self {
        Self {
            bet_sync_interval_ms: self.bet_sync_interval_ms,
            market_sync_interval_ms: self.market_sync_interval_ms,
            enable_immediate_sync: self.enable_immediate_sync,
            max_retries: self.max_retries,
        }
    }
}
