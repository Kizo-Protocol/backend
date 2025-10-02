use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct MarketCreatedEvent {
    pub market_id: String,
    pub question: String,
    pub end_time: String,
    pub yield_protocol_addr: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct BetPlacedEvent {
    pub bet_id: String,
    pub market_id: String,
    pub user: String,
    pub position: bool,
    pub amount: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct MarketResolvedEvent {
    pub market_id: String,
    pub outcome: bool,
    pub total_yield_earned: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct WinningsClaimedEvent {
    pub bet_id: String,
    pub user: String,
    pub winning_amount: String,
    pub yield_share: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct YieldDepositedEvent {
    pub market_id: String,
    pub amount: String,
    pub protocol_addr: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ProtocolFeeCollectedEvent {
    pub market_id: String,
    pub fee_amount: String,
}

#[allow(dead_code)]
pub struct EventIndexer {
    pool: PgPool,
    node_url: String,
    module_address: String,
    last_processed_version: u64,
}

#[allow(dead_code)]
impl EventIndexer {
    pub fn new(pool: PgPool) -> Result<Self> {
        let node_url = std::env::var("APTOS_NODE_URL")
            .unwrap_or_else(|_| "https://fullnode.testnet.aptoslabs.com/v1".to_string());

        let module_address = std::env::var("APTOS_MODULE_ADDRESS")
            .map_err(|_| anyhow!("APTOS_MODULE_ADDRESS environment variable is required"))?;

        Ok(Self {
            pool,
            node_url,
            module_address,
            last_processed_version: 0,
        })
    }

    pub async fn start_indexing(&mut self) -> Result<()> {
        info!("Starting event indexer...");

        self.last_processed_version = self.get_last_processed_version().await?;

        info!("Resuming from version: {}", self.last_processed_version);

        loop {
            match self.process_events_batch().await {
                Ok(processed) => {
                    if processed > 0 {
                        info!("Processed {} events", processed);
                    }
                }
                Err(e) => {
                    error!("Error processing events: {}", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    async fn process_events_batch(&mut self) -> Result<usize> {
        let events = self
            .fetch_events_from_node(self.last_processed_version)
            .await?;

        if events.is_empty() {
            return Ok(0);
        }

        let mut processed_count = 0;

        for event in events {
            match self.process_single_event(&event).await {
                Ok(_) => {
                    processed_count += 1;
                    self.last_processed_version = event.version;
                }
                Err(e) => {
                    error!("Failed to process event: {:?}, error: {}", event, e);
                }
            }
        }

        if processed_count > 0 {
            self.update_last_processed_version(self.last_processed_version)
                .await?;
        }

        Ok(processed_count)
    }

    async fn fetch_events_from_node(&self, _from_version: u64) -> Result<Vec<AptosEvent>> {
        Ok(vec![])
    }

    async fn process_single_event(&self, event: &AptosEvent) -> Result<()> {
        match event.event_type.as_str() {
            "MarketCreatedEvent" => {
                self.handle_market_created_event(event).await?;
            }
            "BetPlacedEvent" => {
                self.handle_bet_placed_event(event).await?;
            }
            "MarketResolvedEvent" => {
                self.handle_market_resolved_event(event).await?;
            }
            "WinningsClaimedEvent" => {
                self.handle_winnings_claimed_event(event).await?;
            }
            "YieldDepositedEvent" => {
                self.handle_yield_deposited_event(event).await?;
            }
            "ProtocolFeeCollectedEvent" => {
                self.handle_protocol_fee_collected_event(event).await?;
            }
            _ => {
                warn!("Unknown event type: {}", event.event_type);
            }
        }

        Ok(())
    }

    async fn handle_market_created_event(&self, event: &AptosEvent) -> Result<()> {
        let data: MarketCreatedEvent = serde_json::from_value(event.data.clone())?;

        info!(
            "Processing MarketCreatedEvent: market_id={}",
            data.market_id
        );

        let market_id: i64 = data.market_id.parse()?;
        let end_time = chrono::DateTime::parse_from_rfc3339(&data.end_time)?;

        let existing = sqlx::query!(
            r#"SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1"#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            info!("Market {} already exists, updating", market_id);

            sqlx::query!(
                r#"
                UPDATE markets_extended
                SET "endDate" = $1, "updatedAt" = NOW()
                WHERE "blockchainMarketId" = $2
                "#,
                end_time.naive_utc(),
                market_id
            )
            .execute(&self.pool)
            .await?;
        } else {
            let id = Uuid::new_v4().to_string();

            sqlx::query!(
                r#"
                INSERT INTO markets_extended (
                    id, "blockchainMarketId", question, "endDate",
                    status, platform, "createdAt", "updatedAt"
                )
                VALUES ($1, $2, $3, $4, 'active', 'aptos', NOW(), NOW())
                "#,
                id,
                market_id,
                data.question,
                end_time.naive_utc()
            )
            .execute(&self.pool)
            .await?;

            info!("Created market record for blockchain market {}", market_id);
        }

        Ok(())
    }

    async fn handle_bet_placed_event(&self, event: &AptosEvent) -> Result<()> {
        let data: BetPlacedEvent = serde_json::from_value(event.data.clone())?;

        info!(
            "Processing BetPlacedEvent: bet_id={}, market_id={}",
            data.bet_id, data.market_id
        );

        let bet_id_i64: i64 = data.bet_id.parse()?;
        let market_id_i64: i64 = data.market_id.parse()?;
        let amount = data.amount.parse::<sqlx::types::BigDecimal>()?;

        let market = sqlx::query!(
            r#"SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1"#,
            market_id_i64
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("Market not found for blockchain ID {}", market_id_i64))?;

        let existing = sqlx::query!(
            r#"SELECT id FROM bets_extended WHERE "blockchainBetId" = $1"#,
            bet_id_i64
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_none() {
            let bet_uuid = Uuid::new_v4().to_string();

            sqlx::query!(
                r#"
                INSERT INTO bets_extended (
                    id, "userId", "marketId", "blockchainBetId", position, amount,
                    odds, status, "createdAt", "updatedAt"
                )
                VALUES ($1, $2, $3, $4, $5, $6, '1.0', 'active', NOW(), NOW())
                "#,
                bet_uuid,
                data.user,
                market.id,
                bet_id_i64,
                data.position,
                amount
            )
            .execute(&self.pool)
            .await?;

            if data.position {
                sqlx::query!(
                    r#"
                    UPDATE markets_extended
                    SET "yesPoolSize" = "yesPoolSize" + $1,
                        "totalPoolSize" = "totalPoolSize" + $1,
                        "countYes" = "countYes" + 1,
                        "updatedAt" = NOW()
                    WHERE id = $2
                    "#,
                    amount,
                    market.id
                )
                .execute(&self.pool)
                .await?;
            } else {
                sqlx::query!(
                    r#"
                    UPDATE markets_extended
                    SET "noPoolSize" = "noPoolSize" + $1,
                        "totalPoolSize" = "totalPoolSize" + $1,
                        "countNo" = "countNo" + 1,
                        "updatedAt" = NOW()
                    WHERE id = $2
                    "#,
                    amount,
                    market.id
                )
                .execute(&self.pool)
                .await?;
            }

            info!("Created bet record for blockchain bet {}", bet_id_i64);
        }

        Ok(())
    }

    async fn handle_market_resolved_event(&self, event: &AptosEvent) -> Result<()> {
        let data: MarketResolvedEvent = serde_json::from_value(event.data.clone())?;

        info!(
            "Processing MarketResolvedEvent: market_id={}, outcome={}",
            data.market_id, data.outcome
        );

        let market_id: i64 = data.market_id.parse()?;
        let yield_earned = data.total_yield_earned.parse::<sqlx::types::BigDecimal>()?;

        sqlx::query!(
            r#"
            UPDATE markets_extended
            SET status = 'resolved',
                result = $1,
                "totalYieldEarned" = $2,
                "resolutionDate" = NOW(),
                "updatedAt" = NOW()
            WHERE "blockchainMarketId" = $3
            "#,
            data.outcome,
            yield_earned,
            market_id
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
            market_id,
            data.outcome
        )
        .execute(&self.pool)
        .await?;

        info!(
            "Market {} resolved with outcome: {}",
            market_id,
            if data.outcome { "YES" } else { "NO" }
        );

        Ok(())
    }

    async fn handle_winnings_claimed_event(&self, event: &AptosEvent) -> Result<()> {
        let data: WinningsClaimedEvent = serde_json::from_value(event.data.clone())?;

        info!(
            "Processing WinningsClaimedEvent: bet_id={}, user={}",
            data.bet_id, data.user
        );

        let bet_id: i64 = data.bet_id.parse()?;
        let winning = data.winning_amount.parse::<sqlx::types::BigDecimal>()?;
        let yield_share = data.yield_share.parse::<sqlx::types::BigDecimal>()?;
        let total_payout = winning + yield_share;

        sqlx::query!(
            r#"
            UPDATE bets_extended
            SET status = 'claimed',
                payout = $1,
                "updatedAt" = NOW()
            WHERE "blockchainBetId" = $2
            "#,
            total_payout,
            bet_id
        )
        .execute(&self.pool)
        .await?;

        info!("Bet {} claimed by user {}", bet_id, data.user);

        Ok(())
    }

    async fn handle_yield_deposited_event(&self, event: &AptosEvent) -> Result<()> {
        let data: YieldDepositedEvent = serde_json::from_value(event.data.clone())?;

        info!(
            "Processing YieldDepositedEvent: market_id={}, amount={}",
            data.market_id, data.amount
        );

        Ok(())
    }

    async fn handle_protocol_fee_collected_event(&self, event: &AptosEvent) -> Result<()> {
        let data: ProtocolFeeCollectedEvent = serde_json::from_value(event.data.clone())?;

        info!(
            "Processing ProtocolFeeCollectedEvent: market_id={}, fee={}",
            data.market_id, data.fee_amount
        );

        Ok(())
    }

    async fn get_last_processed_version(&self) -> Result<u64> {
        let result = sqlx::query!(
            "SELECT MAX(last_processed_version) as version FROM indexer_state WHERE indexer_name = 'event_indexer'"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|r| r.version).unwrap_or(0) as u64)
    }

    async fn update_last_processed_version(&self, version: u64) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO indexer_state (indexer_name, last_processed_version, updated_at)
            VALUES ('event_indexer', $1, NOW())
            ON CONFLICT (indexer_name)
            DO UPDATE SET last_processed_version = $1, updated_at = NOW()
            "#,
            version as i64
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AptosEvent {
    version: u64,
    event_type: String,
    data: serde_json::Value,
}
