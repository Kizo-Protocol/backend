use anyhow::Result;
use sqlx::PgPool;
use tracing::{info};

#[allow(dead_code)]
pub struct MarketService {
    pool: PgPool,
}

impl MarketService {
    #[allow(dead_code)]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }


    #[allow(dead_code)]
    pub async fn sync_market_from_indexer(&self, market_id: i64) -> Result<()> {
        info!("Syncing market {} from indexer", market_id);




        Ok(())
    }


    #[allow(dead_code)]
    pub async fn get_market_statistics(&self, market_id: i64) -> Result<MarketStatistics> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(b.bet_id) as total_bets,
                COALESCE(SUM(b.amount), 0) as total_volume,
                COALESCE(SUM(CASE WHEN b.position = true THEN b.amount ELSE 0 END), 0) as yes_volume,
                COALESCE(SUM(CASE WHEN b.position = false THEN b.amount ELSE 0 END), 0) as no_volume,
                COUNT(DISTINCT b.user_addr) as unique_bettors
            FROM bets b
            WHERE b.market_id = $1
            "#,
            market_id
        )
        .fetch_one(&self.pool)
        .await?;


        Ok(MarketStatistics {
            market_id,
            total_bets: stats.total_bets.unwrap_or(0),
            total_volume: stats.total_volume.and_then(|v| v.to_string().parse().ok()).unwrap_or(0),
            yes_volume: stats.yes_volume.and_then(|v| v.to_string().parse().ok()).unwrap_or(0),
            no_volume: stats.no_volume.and_then(|v| v.to_string().parse().ok()).unwrap_or(0),
            unique_bettors: stats.unique_bettors.unwrap_or(0),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketStatistics {
    pub market_id: i64,
    pub total_bets: i64,
    pub total_volume: i64,
    pub yes_volume: i64,
    pub no_volume: i64,
    pub unique_bettors: i64,
}