use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tracing::{debug, info};

use crate::models::*;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to database: {}", database_url);

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .context("Failed to connect to database")?;

        info!("Database connection pool established");

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn get_markets(&self, params: &MarketQueryParams) -> Result<Vec<MarketExtended>> {
        let status_filter = match params.status {
            MarketStatus::Active => " WHERE status = 'active'",
            MarketStatus::Resolved => " WHERE status != 'active'",
            MarketStatus::All => "",
        };

        let sort_column = match params.sort_by {
            MarketSortBy::EndTime => "\"endDate\"",
            MarketSortBy::TransactionVersion => "\"updatedAt\"",
        };

        let order = match params.order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };

        let query = format!(
            "SELECT * FROM markets_extended{} ORDER BY {} {} LIMIT $1 OFFSET $2",
            status_filter, sort_column, order
        );

        let markets = sqlx::query_as::<_, MarketExtended>(&query)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch markets")?;

        debug!("Fetched {} markets", markets.len());
        Ok(markets)
    }

    pub async fn get_market_extended_by_id(
        &self,
        market_id: &str,
    ) -> Result<Option<MarketExtended>> {
        let market = sqlx::query_as::<_, MarketExtended>(
            "SELECT * FROM markets_extended WHERE id = $1 OR \"marketId\" = $1 OR \"adjTicker\" = $1"
        )
        .bind(market_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch market")?;

        Ok(market)
    }

    pub async fn get_market_extended_by_blockchain_id(
        &self,
        blockchain_market_id: i64,
    ) -> Result<Option<MarketExtended>> {
        let market = sqlx::query_as::<_, MarketExtended>(
            "SELECT * FROM markets_extended WHERE \"blockchainMarketId\" = $1",
        )
        .bind(blockchain_market_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch market")?;

        Ok(market)
    }

    pub async fn count_markets(&self, params: &MarketQueryParams) -> Result<i64> {
        let status_filter = match params.status {
            MarketStatus::Active => " WHERE status = 'active'",
            MarketStatus::Resolved => " WHERE status != 'active'",
            MarketStatus::All => "",
        };

        let query = format!(
            "SELECT COUNT(*) as count FROM markets_extended{}",
            status_filter
        );

        let row = sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count markets")?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    pub async fn get_market_by_id(&self, market_id: i64) -> Result<Option<Market>> {
        let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE market_id = $1")
            .bind(market_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to fetch market")?;

        Ok(market)
    }

    pub async fn get_market_stats(&self, market_id: i64) -> Result<MarketStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(COUNT(b.bet_id), 0) as total_bets,
                COALESCE(SUM(b.amount), 0) as total_volume,
                COALESCE(SUM(CASE WHEN b.position = true THEN b.amount ELSE 0 END), 0) as yes_volume,
                COALESCE(SUM(CASE WHEN b.position = false THEN b.amount ELSE 0 END), 0) as no_volume,
                COALESCE(COUNT(DISTINCT b.user_addr), 0) as unique_bettors
            FROM bets b
            WHERE b.market_id = $1
            "#
        )
        .bind(market_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch market stats")?;

        let total_bets: i64 = row.try_get("total_bets")?;
        let total_volume: sqlx::types::BigDecimal = row
            .try_get::<sqlx::types::BigDecimal, _>("total_volume")
            .unwrap_or_else(|_| sqlx::types::BigDecimal::from(0));
        let yes_volume: sqlx::types::BigDecimal = row
            .try_get::<sqlx::types::BigDecimal, _>("yes_volume")
            .unwrap_or_else(|_| sqlx::types::BigDecimal::from(0));
        let no_volume: sqlx::types::BigDecimal = row
            .try_get::<sqlx::types::BigDecimal, _>("no_volume")
            .unwrap_or_else(|_| sqlx::types::BigDecimal::from(0));
        let unique_bettors: i64 = row.try_get("unique_bettors")?;

        let total_volume_f64 = total_volume.to_string().parse::<f64>().unwrap_or(0.0);
        let yes_volume_f64 = yes_volume.to_string().parse::<f64>().unwrap_or(0.0);
        let no_volume_f64 = no_volume.to_string().parse::<f64>().unwrap_or(0.0);

        let yes_percentage = if total_volume_f64 > 0.0 {
            (yes_volume_f64 / total_volume_f64) * 100.0
        } else {
            0.0
        };

        let no_percentage = if total_volume_f64 > 0.0 {
            (no_volume_f64 / total_volume_f64) * 100.0
        } else {
            0.0
        };

        Ok(MarketStats {
            market_id,
            total_bets,
            total_volume: total_volume.to_string(),
            yes_volume: yes_volume.to_string(),
            no_volume: no_volume.to_string(),
            yes_percentage,
            no_percentage,
            unique_bettors,
        })
    }

    pub async fn get_platform_stats(&self) -> Result<PlatformStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_markets,
                COUNT(*) FILTER (WHERE resolved = false OR resolved IS NULL) as active_markets,
                COUNT(*) FILTER (WHERE resolved = true) as resolved_markets
            FROM markets
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch platform market stats")?;

        let total_markets: i64 = row.try_get("total_markets")?;
        let active_markets: i64 = row.try_get("active_markets")?;
        let resolved_markets: i64 = row.try_get("resolved_markets")?;

        let bet_row = sqlx::query(
            r#"
            SELECT
                COALESCE(COUNT(*), 0) as total_bets,
                COALESCE(SUM(amount), 0) as total_volume,
                COALESCE(COUNT(DISTINCT user_addr), 0) as unique_users
            FROM bets
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch platform bet stats")?;

        let total_bets: i64 = bet_row.try_get("total_bets")?;
        let total_volume: sqlx::types::BigDecimal = bet_row
            .try_get::<sqlx::types::BigDecimal, _>("total_volume")
            .unwrap_or_else(|_| sqlx::types::BigDecimal::from(0));
        let unique_users: i64 = bet_row.try_get("unique_users")?;

        let yield_row = sqlx::query(
            "SELECT COALESCE(SUM(total_yield_earned), 0) as total_yield FROM market_resolutions",
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch yield stats")?;

        let total_yield_earned: sqlx::types::BigDecimal = yield_row
            .try_get::<sqlx::types::BigDecimal, _>("total_yield")
            .unwrap_or_else(|_| sqlx::types::BigDecimal::from(0));

        Ok(PlatformStats {
            total_markets,
            active_markets,
            resolved_markets,
            total_bets,
            total_volume: total_volume.to_string(),
            unique_users,
            total_yield_earned: total_yield_earned.to_string(),
        })
    }

    pub async fn get_bets(&self, params: &PaginationParams) -> Result<Vec<Bet>> {
        let bets = sqlx::query_as::<_, Bet>(
            "SELECT * FROM bets ORDER BY transaction_version DESC LIMIT $1 OFFSET $2",
        )
        .bind(params.limit)
        .bind(params.offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch bets")?;

        Ok(bets)
    }

    pub async fn get_bet_by_id(&self, bet_id: i64) -> Result<Option<Bet>> {
        let bet = sqlx::query_as::<_, Bet>("SELECT * FROM bets WHERE bet_id = $1")
            .bind(bet_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to fetch bet")?;

        Ok(bet)
    }

    pub async fn get_bets_by_user(
        &self,
        user_addr: &str,
        params: &PaginationParams,
    ) -> Result<Vec<Bet>> {
        let bets = sqlx::query_as::<_, Bet>(
            "SELECT * FROM bets WHERE user_addr = $1 ORDER BY transaction_version DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_addr)
        .bind(params.limit)
        .bind(params.offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch user bets")?;

        Ok(bets)
    }

    pub async fn get_bets_by_market(
        &self,
        market_id: i64,
        params: &PaginationParams,
    ) -> Result<Vec<Bet>> {
        let bets = sqlx::query_as::<_, Bet>(
            "SELECT * FROM bets WHERE market_id = $1 ORDER BY transaction_version DESC LIMIT $2 OFFSET $3"
        )
        .bind(market_id)
        .bind(params.limit)
        .bind(params.offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch market bets")?;

        Ok(bets)
    }

    pub async fn get_user_stats(&self, user_addr: &str) -> Result<UserStats> {
        let bet_row = sqlx::query(
            r#"
            SELECT
                COALESCE(COUNT(*), 0) as total_bets,
                COALESCE(SUM(amount), 0) as total_wagered,
                COALESCE(COUNT(DISTINCT market_id), 0) as markets_participated
            FROM bets
            WHERE user_addr = $1
            "#,
        )
        .bind(user_addr)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch user bet stats")?;

        let total_bets: i64 = bet_row.try_get("total_bets")?;
        let total_wagered: i64 = bet_row.try_get("total_wagered")?;
        let markets_participated: i64 = bet_row.try_get("markets_participated")?;

        let outcome_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE b.claimed = true AND b.winning_amount > 0) as wins,
                COUNT(*) FILTER (WHERE m.resolved = true AND b.claimed = false) as losses,
                COUNT(*) FILTER (WHERE m.resolved = false OR m.resolved IS NULL) as pending
            FROM bets b
            JOIN markets m ON b.market_id = m.market_id
            WHERE b.user_addr = $1
            "#,
        )
        .bind(user_addr)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch user outcome stats")?;

        let wins: i64 = outcome_row.try_get("wins")?;
        let losses: i64 = outcome_row.try_get("losses")?;
        let pending: i64 = outcome_row.try_get("pending")?;

        let winnings_row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(winning_amount), 0) as total_winnings,
                COALESCE(SUM(yield_share), 0) as total_yield_earned
            FROM winnings_claims
            WHERE user_addr = $1
            "#,
        )
        .bind(user_addr)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch user winnings")?;

        let total_winnings: i64 = winnings_row.try_get("total_winnings")?;
        let total_yield_earned: i64 = winnings_row.try_get("total_yield_earned")?;

        Ok(UserStats {
            user_addr: user_addr.to_string(),
            total_bets,
            total_wagered: total_wagered.to_string(),
            markets_participated,
            wins,
            losses,
            pending,
            total_winnings: total_winnings.to_string(),
            total_yield_earned: total_yield_earned.to_string(),
        })
    }

    pub async fn get_recent_bets(&self, limit: i64) -> Result<Vec<Bet>> {
        let bets = sqlx::query_as::<_, Bet>(
            "SELECT * FROM bets ORDER BY transaction_version DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch recent bets")?;

        Ok(bets)
    }

    pub async fn get_sync_status(&self) -> Result<Vec<SyncStatus>> {
        let statuses = vec![
            self.get_table_status("markets", "MarketCreated").await?,
            self.get_table_status("bets", "BetPlaced").await?,
            self.get_table_status("market_resolutions", "MarketResolved")
                .await?,
            self.get_table_status("winnings_claims", "WinningsClaimed")
                .await?,
            self.get_table_status("yield_deposits", "YieldDeposited")
                .await?,
            self.get_table_status("protocol_fees", "ProtocolFeeCollected")
                .await?,
        ];

        Ok(statuses)
    }

    async fn get_table_status(&self, table_name: &str, event_type: &str) -> Result<SyncStatus> {
        let query = format!(
            "SELECT MAX(transaction_version) as max_version, MAX(transaction_block_height) as max_height, MAX(inserted_at) as last_sync FROM {}",
            table_name
        );

        let row = sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .context(format!("Failed to fetch sync status for {}", table_name))?;

        let max_version: Option<i64> = row.try_get("max_version")?;
        let max_height: Option<i64> = row.try_get("max_height")?;
        let last_sync: Option<chrono::NaiveDateTime> = row.try_get("last_sync")?;

        Ok(SyncStatus {
            last_indexed_version: max_version.unwrap_or(0),
            last_indexed_block_height: max_height.unwrap_or(0),
            event_type: event_type.to_string(),
            last_sync_time: last_sync.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        })
    }

    pub async fn health_check(&self) -> Result<bool> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;

        Ok(true)
    }
}
