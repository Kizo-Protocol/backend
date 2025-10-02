use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgListener, PgPool};
use std::time::Instant;
use tracing::{error, info, warn};

use super::blockchain_sync::BlockchainSyncService;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum DatabaseEvent {
    NewBet(BetEventData),
    NewMarket(MarketEventData),
    MarketResolution(MarketResolutionEventData),
    WinningsClaim(WinningsClaimEventData),
    YieldDeposit(YieldDepositEventData),
    ProtocolFee(ProtocolFeeEventData),
    BlockchainEvent(GenericEventData),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BetEventData {
    #[serde(default)]
    pub operation: Option<String>,
    pub bet_id: i64,
    pub market_id: i64,
    pub user_addr: String,
    pub position: bool,
    pub amount: i64,
    #[serde(default)]
    pub claimed: Option<bool>,
    pub transaction_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketEventData {
    #[serde(default)]
    pub operation: Option<String>,
    pub market_id: i64,
    pub question: String,
    pub end_time: i64,
    #[serde(default)]
    pub resolved: Option<bool>,
    #[serde(default)]
    pub outcome: Option<bool>,
    pub yield_protocol_addr: String,
    pub transaction_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketResolutionEventData {
    pub market_id: i64,
    pub outcome: bool,
    pub total_yes_pool: i64,
    pub total_no_pool: i64,
    pub transaction_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WinningsClaimEventData {
    pub bet_id: i64,
    pub user_addr: String,
    pub winning_amount: i64,
    pub yield_share: i64,
    pub transaction_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YieldDepositEventData {
    pub market_id: i64,
    pub amount: i64,
    pub protocol_addr: String,
    pub transaction_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolFeeEventData {
    pub market_id: i64,
    pub fee_amount: i64,
    pub transaction_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenericEventData {
    pub event_type: String,
    pub market_id: Option<i64>,
    pub transaction_version: i64,
    pub event_data: serde_json::Value,
    pub created_at: Option<String>,
}

pub struct DbEventListener {
    pool: PgPool,
    blockchain_sync: BlockchainSyncService,
}

impl DbEventListener {
    pub fn new(pool: PgPool) -> Self {
        let blockchain_sync = BlockchainSyncService::new(pool.clone());
        Self {
            pool,
            blockchain_sync,
        }
    }

    pub async fn start_listening(self) -> Result<()> {
        info!("🎧 Starting database event listener...");

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let mut listener = PgListener::connect(&database_url).await?;

        listener.listen("bet_event").await?;
        listener.listen("market_event").await?;

        listener.listen("new_bet_event").await?;
        listener.listen("new_market_event").await?;

        listener.listen("market_resolution_event").await?;
        listener.listen("winnings_claim_event").await?;
        listener.listen("yield_deposit_event").await?;
        listener.listen("protocol_fee_event").await?;
        listener.listen("blockchain_event").await?;

        info!("✅ Database event listener started and subscribed to all channels");
        info!("👂 Listening for INSERTUPDATE events on: bets, markets, resolutions, claims, yields, fees");
        info!(
            "⚡ Real-time updates will be processed immediately when indexer adds OR modifies data"
        );

        loop {
            match listener.recv().await {
                Ok(notification) => {
                    let start = Instant::now();
                    let channel = notification.channel();
                    let payload = notification.payload();

                    info!("📨 Received event on channel: {}", channel);

                    match self.process_notification(channel, payload).await {
                        Ok(_) => {
                            let duration = start.elapsed().as_millis() as i32;
                            info!("✅ Event processed successfully in {}ms", duration);

                            if let Err(e) = self
                                .log_event_processing(channel, payload, "success", None, duration)
                                .await
                            {
                                warn!("Failed to log event processing: {}", e);
                            }
                        }
                        Err(e) => {
                            let duration = start.elapsed().as_millis() as i32;
                            error!("❌ Failed to process event: {}", e);

                            if let Err(log_err) = self
                                .log_event_processing(
                                    channel,
                                    payload,
                                    "error",
                                    Some(&e.to_string()),
                                    duration,
                                )
                                .await
                            {
                                warn!("Failed to log event error: {}", log_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving notification: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_notification(&self, channel: &str, payload: &str) -> Result<()> {
        match channel {
            "bet_event" => {
                let event_data: BetEventData = serde_json::from_str(payload)?;
                let operation = event_data.operation.as_deref().unwrap_or("UNKNOWN");
                info!(
                    "📌 Bet event detected: {} on bet_id={}",
                    operation, event_data.bet_id
                );
                self.handle_bet_event(event_data).await?;
            }
            "market_event" => {
                let event_data: MarketEventData = serde_json::from_str(payload)?;
                let operation = event_data.operation.as_deref().unwrap_or("UNKNOWN");
                info!(
                    "📌 Market event detected: {} on market_id={}",
                    operation, event_data.market_id
                );
                self.handle_market_event(event_data).await?;
            }
            "new_bet_event" => {
                let event_data: BetEventData = serde_json::from_str(payload)?;
                self.handle_bet_event(event_data).await?;
            }
            "new_market_event" => {
                let event_data: MarketEventData = serde_json::from_str(payload)?;
                self.handle_market_event(event_data).await?;
            }
            "market_resolution_event" => {
                let event_data: MarketResolutionEventData = serde_json::from_str(payload)?;
                self.handle_market_resolution(event_data).await?;
            }
            "winnings_claim_event" => {
                let event_data: WinningsClaimEventData = serde_json::from_str(payload)?;
                self.handle_winnings_claim(event_data).await?;
            }
            "yield_deposit_event" => {
                let event_data: YieldDepositEventData = serde_json::from_str(payload)?;
                self.handle_yield_deposit(event_data).await?;
            }
            "protocol_fee_event" => {
                let event_data: ProtocolFeeEventData = serde_json::from_str(payload)?;
                self.handle_protocol_fee(event_data).await?;
            }
            "blockchain_event" => {
                let event_data: GenericEventData = serde_json::from_str(payload)?;
                self.handle_blockchain_event(event_data).await?;
            }
            _ => {
                warn!("Unknown event channel: {}", channel);
            }
        }

        Ok(())
    }

    async fn handle_bet_event(&self, event: BetEventData) -> Result<()> {
        let operation = event.operation.as_deref().unwrap_or("INSERT");
        info!(
            "🎲 Processing {} bet: bet_id={}, market_id={}, amount={}",
            operation, event.bet_id, event.market_id, event.amount
        );

        self.blockchain_sync.sync_bets().await?;

        let market_id_str = event.market_id.to_string();
        self.blockchain_sync
            .update_market_stats_for_market(&market_id_str)
            .await?;

        info!("✅ Bet event processed and market stats updated");
        Ok(())
    }

    async fn handle_market_event(&self, event: MarketEventData) -> Result<()> {
        let operation = event.operation.as_deref().unwrap_or("INSERT");
        info!(
            "🏪 Processing {} market: market_id={}, question={}",
            operation, event.market_id, event.question
        );

        self.blockchain_sync.sync_markets().await?;

        if operation == "UPDATE" && event.resolved.unwrap_or(false) {
            info!(
                "🎯 Market resolution detected for market_id={}",
                event.market_id
            );
        }

        info!("✅ Market event processed");
        Ok(())
    }

    async fn handle_market_resolution(&self, event: MarketResolutionEventData) -> Result<()> {
        info!(
            "🎯 Processing market resolution: market_id={}, outcome={}",
            event.market_id, event.outcome
        );

        sqlx::query!(
            r#"
            UPDATE markets_extended
            SET status = 'resolved',
                result = $1,
                "resolutionDate" = NOW(),
                "updatedAt" = NOW()
            WHERE "blockchainMarketId" = $2
            "#,
            event.outcome,
            event.market_id
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!(
            r#"
            UPDATE bets_extended
            SET status = 'lost', "updatedAt" = NOW()
            WHERE "marketId" IN (
                SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1
            )
            AND position != $2
            AND status = 'active'
            "#,
            event.market_id,
            event.outcome
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!(
            r#"
            UPDATE bets_extended
            SET status = 'won', "updatedAt" = NOW()
            WHERE "marketId" IN (
                SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1
            )
            AND position = $2
            AND status = 'active'
            "#,
            event.market_id,
            event.outcome
        )
        .execute(&self.pool)
        .await?;

        info!("✅ Market resolution processed");
        Ok(())
    }

    async fn handle_winnings_claim(&self, event: WinningsClaimEventData) -> Result<()> {
        info!(
            "💰 Processing winnings claim: bet_id={}, user={}, amount={}",
            event.bet_id, event.user_addr, event.winning_amount
        );

        let total_payout = event.winning_amount + event.yield_share;
        let total_payout_decimal = sqlx::types::BigDecimal::from(total_payout);

        sqlx::query!(
            r#"
            UPDATE bets_extended
            SET status = 'claimed',
                payout = $1,
                "updatedAt" = NOW()
            WHERE "blockchainBetId" = $2
            "#,
            total_payout_decimal,
            event.bet_id
        )
        .execute(&self.pool)
        .await?;

        info!("✅ Winnings claim processed");
        Ok(())
    }

    async fn handle_yield_deposit(&self, event: YieldDepositEventData) -> Result<()> {
        info!(
            "📈 Processing yield deposit: market_id={}, amount={}",
            event.market_id, event.amount
        );

        let amount_decimal = sqlx::types::BigDecimal::from(event.amount);

        sqlx::query!(
            r#"
            UPDATE markets_extended
            SET "totalYieldEarned" = "totalYieldEarned" + $1,
                "updatedAt" = NOW()
            WHERE "blockchainMarketId" = $2
            "#,
            amount_decimal,
            event.market_id
        )
        .execute(&self.pool)
        .await?;

        info!("✅ Yield deposit processed");
        Ok(())
    }

    async fn handle_protocol_fee(&self, event: ProtocolFeeEventData) -> Result<()> {
        info!(
            "💵 Processing protocol fee: market_id={}, amount={}",
            event.market_id, event.fee_amount
        );

        info!("✅ Protocol fee processed");
        Ok(())
    }

    async fn handle_blockchain_event(&self, event: GenericEventData) -> Result<()> {
        info!("⛓️  Processing blockchain event: type={}", event.event_type);

        info!("✅ Blockchain event processed");
        Ok(())
    }

    async fn log_event_processing(
        &self,
        event_type: &str,
        event_data: &str,
        status: &str,
        error_message: Option<&str>,
        duration_ms: i32,
    ) -> Result<()> {
        let event_data_json: serde_json::Value = serde_json::from_str(event_data)
            .unwrap_or_else(|_| serde_json::json!({"raw": event_data}));

        let transaction_version: Option<i64> = event_data_json
            .get("transaction_version")
            .and_then(|v| v.as_i64());

        sqlx::query!(
            r#"
            INSERT INTO event_processing_log
                (event_type, event_data, transaction_version, processing_status, error_message, processing_duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            event_type,
            event_data_json,
            transaction_version,
            status,
            error_message,
            duration_ms
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_event_stats(&self) -> Result<Vec<EventStats>> {
        let stats = sqlx::query_as!(
            EventStats,
            r#"
            SELECT
                event_type,
                total_processed,
                successful,
                errors,
                avg_duration_ms,
                last_processed_at
            FROM event_processing_stats
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventStats {
    pub event_type: Option<String>,
    pub total_processed: Option<i64>,
    pub successful: Option<i64>,
    pub errors: Option<i64>,
    pub avg_duration_ms: Option<sqlx::types::BigDecimal>,
    pub last_processed_at: Option<chrono::NaiveDateTime>,
}
