use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaceBetParams {
    pub market_identifier: String,
    pub user_address: String,
    pub position: bool,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaceBetResult {
    pub bet_id: String,
    pub blockchain_bet_id: u64,
    pub market_id: String,
    pub blockchain_market_id: u64,
    pub position: bool,
    pub amount: String,
    pub tx_hash: String,
    pub user_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimWinningsParams {
    pub market_identifier: String,
    pub user_address: String,
    pub bet_index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimWinningsResult {
    pub bet_id: String,
    pub winning_amount: String,
    pub yield_share: String,
    pub total_claimed: String,
    pub tx_hash: String,
}

pub struct BettingService {
    pool: PgPool,
    #[allow(dead_code)]
    node_url: String,
    #[allow(dead_code)]
    module_address: String,
    #[allow(dead_code)]
    module_name: String,
}

impl BettingService {
    pub fn new(pool: PgPool) -> Result<Self> {
        let node_url = std::env::var("APTOS_NODE_URL")
            .unwrap_or_else(|_| "https://fullnode.testnet.aptoslabs.com/v1".to_string());

        let module_address = std::env::var("APTOS_MODULE_ADDRESS")
            .map_err(|_| anyhow!("APTOS_MODULE_ADDRESS environment variable is required"))?;

        let module_name = std::env::var("APTOS_MODULE_NAME")
            .unwrap_or_else(|_| "kizo_prediction_market".to_string());

        Ok(Self {
            pool,
            node_url,
            module_address,
            module_name,
        })
    }

    pub async fn place_bet(&self, params: PlaceBetParams) -> Result<PlaceBetResult> {
        info!("Placing bet on market: {}", params.market_identifier);

        let market = self
            .get_market_by_identifier(&params.market_identifier)
            .await?;

        if market.blockchain_market_id.is_none() {
            return Err(anyhow!("Market is not on blockchain yet"));
        }

        let blockchain_market_id = market.blockchain_market_id.unwrap() as u64;

        if market.status != "active" {
            return Err(anyhow!("Market is not active"));
        }

        let amount_u64: u64 = params
            .amount
            .parse()
            .map_err(|_| anyhow!("Invalid amount"))?;

        if amount_u64 == 0 {
            return Err(anyhow!("Amount must be greater than 0"));
        }

        let contract_addr =
            std::env::var("APTOS_CONTRACT_ADDRESS").unwrap_or_else(|_| self.module_address.clone());

        let (tx_hash, blockchain_bet_id) = self
            .submit_bet_transaction(
                &params.user_address,
                &contract_addr,
                blockchain_market_id,
                params.position,
                amount_u64,
            )
            .await?;

        info!(
            "Bet transaction submitted: {} with bet ID {}",
            tx_hash, blockchain_bet_id
        );

        let odds = self
            .calculate_bet_odds(&market.id, params.position, &params.amount)
            .await?;
        let odds_decimal = format!("{:.4}", odds)
            .parse::<sqlx::types::BigDecimal>()
            .unwrap_or_else(|_| "1.0".parse::<sqlx::types::BigDecimal>().unwrap());

        let bet_id = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO bets_extended (
                id, "userId", "marketId", "blockchainBetId", position, amount,
                odds, status, "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', NOW(), NOW())
            "#,
            bet_id,
            params.user_address,
            market.id,
            blockchain_bet_id as i64,
            params.position,
            params
                .amount
                .parse::<sqlx::types::BigDecimal>()
                .unwrap_or_default(),
            odds_decimal,
        )
        .execute(&self.pool)
        .await?;

        self.update_market_pools(&market.id, params.position, &params.amount)
            .await?;

        if let Err(e) = self.trigger_data_sync(&market.id, blockchain_bet_id).await {
            info!(
                "Warning: Failed to trigger automatic sync after bet placement: {}",
                e
            );
        }

        Ok(PlaceBetResult {
            bet_id,
            blockchain_bet_id,
            market_id: market.id,
            blockchain_market_id,
            position: params.position,
            amount: params.amount,
            tx_hash,
            user_address: params.user_address,
        })
    }

    pub async fn claim_winnings(&self, params: ClaimWinningsParams) -> Result<ClaimWinningsResult> {
        info!("Claiming winnings for market: {}", params.market_identifier);

        let market = self
            .get_market_by_identifier(&params.market_identifier)
            .await?;

        if market.blockchain_market_id.is_none() {
            return Err(anyhow!("Market is not on blockchain"));
        }

        if market.status != "resolved" {
            return Err(anyhow!("Market is not resolved yet"));
        }

        let blockchain_market_id = market.blockchain_market_id.unwrap() as u64;

        let contract_addr =
            std::env::var("APTOS_CONTRACT_ADDRESS").unwrap_or_else(|_| self.module_address.clone());

        let (tx_hash, winning_amount, yield_share) = self
            .submit_claim_transaction(
                &params.user_address,
                &contract_addr,
                blockchain_market_id,
                params.bet_index,
            )
            .await?;

        info!("Claim transaction submitted: {}", tx_hash);

        let total_claimed = winning_amount + yield_share;

        sqlx::query!(
            r#"
            UPDATE bets_extended
            SET status = 'claimed',
                payout = $1,
                "updatedAt" = NOW()
            WHERE "marketId" = $2
              AND "userId" = $3
              AND "blockchainBetId" = $4
            "#,
            total_claimed
                .to_string()
                .parse::<sqlx::types::BigDecimal>()
                .unwrap_or_default(),
            market.id,
            params.user_address,
            params.bet_index as i64,
        )
        .execute(&self.pool)
        .await?;

        let bet = sqlx::query!(
            r#"SELECT id FROM bets_extended WHERE "marketId" = $1 AND "userId" = $2 AND "blockchainBetId" = $3"#,
            market.id,
            params.user_address,
            params.bet_index as i64
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ClaimWinningsResult {
            bet_id: bet.id,
            winning_amount: winning_amount.to_string(),
            yield_share: yield_share.to_string(),
            total_claimed: total_claimed.to_string(),
            tx_hash,
        })
    }

    async fn get_market_by_identifier(&self, identifier: &str) -> Result<MarketRecord> {
        let market = sqlx::query_as!(
            MarketRecord,
            r#"
            SELECT id, "blockchainMarketId" as blockchain_market_id, status
            FROM markets_extended
            WHERE id = $1 OR "adjTicker" = $1 OR "blockchainMarketId"::text = $1
            LIMIT 1
            "#,
            identifier
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("Market not found"))?;

        Ok(market)
    }

    async fn submit_bet_transaction(
        &self,
        _user_address: &str,
        contract_addr: &str,
        market_id: u64,
        position: bool,
        amount: u64,
    ) -> Result<(String, u64)> {
        let _private_key = std::env::var("USER_PRIVATE_KEY")
            .map_err(|_| anyhow!("USER_PRIVATE_KEY not configured"))?;

        let function_id = format!("{}::{}::place_bet", self.module_address, self.module_name);

        let payload = json!({
            "type": "entry_function_payload",
            "function": function_id,
            "type_arguments": ["0x1::aptos_coin::AptosCoin"],
            "arguments": [
                contract_addr,
                market_id.to_string(),
                position,
                amount.to_string()
            ]
        });

        info!("Submitting bet transaction: {:?}", payload);

        let mock_bet_id = (chrono::Utc::now().timestamp() % 100000) as u64;
        let mock_tx_hash = format!("0x{:x}", mock_bet_id);

        Ok((mock_tx_hash, mock_bet_id))
    }

    async fn submit_claim_transaction(
        &self,
        _user_address: &str,
        contract_addr: &str,
        market_id: u64,
        bet_index: u64,
    ) -> Result<(String, u64, u64)> {
        let _private_key = std::env::var("USER_PRIVATE_KEY")
            .map_err(|_| anyhow!("USER_PRIVATE_KEY not configured"))?;

        let function_id = format!(
            "{}::{}::claim_winnings",
            self.module_address, self.module_name
        );

        let payload = json!({
            "type": "entry_function_payload",
            "function": function_id,
            "type_arguments": ["0x1::aptos_coin::AptosCoin"],
            "arguments": [
                contract_addr,
                market_id.to_string(),
                bet_index.to_string()
            ]
        });

        info!("Submitting claim transaction: {:?}", payload);

        let mock_tx_hash = format!("0x{:x}", chrono::Utc::now().timestamp());
        let mock_winning = 1000u64;
        let mock_yield = 50u64;

        Ok((mock_tx_hash, mock_winning, mock_yield))
    }

    async fn calculate_bet_odds(
        &self,
        market_id: &str,
        position: bool,
        amount: &str,
    ) -> Result<f64> {
        let market = sqlx::query!(
            r#"
            SELECT "yesPoolSize", "noPoolSize", "totalPoolSize"
            FROM markets_extended
            WHERE id = $1
            "#,
            market_id
        )
        .fetch_one(&self.pool)
        .await?;

        let amount_decimal: f64 = amount
            .parse::<f64>()
            .map_err(|_| anyhow!("Invalid amount format"))?;

        let yes_pool: f64 = market.yesPoolSize.to_string().parse::<f64>().unwrap_or(0.0);
        let no_pool: f64 = market.noPoolSize.to_string().parse::<f64>().unwrap_or(0.0);

        let (final_yes_pool, final_no_pool) = if position {
            (yes_pool + amount_decimal, no_pool)
        } else {
            (yes_pool, no_pool + amount_decimal)
        };

        let total_pool = final_yes_pool + final_no_pool;

        let raw_odds = if position {
            if final_yes_pool > 0.0 {
                total_pool / final_yes_pool
            } else {
                1.0
            }
        } else if final_no_pool > 0.0 {
            total_pool / final_no_pool
        } else {
            1.0
        };

        let odds = raw_odds.max(1.0);

        Ok(odds)
    }

    async fn update_market_pools(
        &self,
        market_id: &str,
        position: bool,
        amount: &str,
    ) -> Result<()> {
        let amount_decimal = amount
            .parse::<sqlx::types::BigDecimal>()
            .unwrap_or_default();

        if position {
            sqlx::query!(
                r#"
                UPDATE markets_extended
                SET "yesPoolSize" = "yesPoolSize" + $1,
                    "totalPoolSize" = "totalPoolSize" + $1,
                    "countYes" = "countYes" + 1,
                    "updatedAt" = NOW()
                WHERE id = $2
                "#,
                amount_decimal,
                market_id
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
                amount_decimal,
                market_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn trigger_data_sync(&self, market_id: &str, bet_id: u64) -> Result<()> {
        info!(
            "Triggering data sync for market {} after bet {}",
            market_id, bet_id
        );

        let realtime_sync = super::realtime_sync::RealtimeSyncService::new(self.pool.clone());

        if let Err(e) = realtime_sync.sync_market_immediately(market_id).await {
            error!("Failed to sync market {} immediately: {}", market_id, e);

            let sync_service =
                super::blockchain_sync::BlockchainSyncService::new(self.pool.clone());
            if let Err(e2) = sync_service.sync_bets().await {
                error!("Fallback sync also failed: {}", e2);
            } else if let Err(e3) = sync_service.update_market_stats().await {
                error!("Failed to update market stats in fallback: {}", e3);
            }
        }

        self.call_webhook_if_configured(market_id, bet_id).await;

        info!(
            "Data sync completed for market {} bet {}",
            market_id, bet_id
        );
        Ok(())
    }

    async fn call_webhook_if_configured(&self, market_id: &str, bet_id: u64) {
        if let Ok(webhook_url) = std::env::var("POST_BET_WEBHOOK_URL") {
            let client = reqwest::Client::new();
            let payload = serde_json::json!({
                "type": "bet_placed",
                "marketId": market_id,
                "betId": bet_id,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            match client.post(&webhook_url).json(&payload).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Successfully notified webhook after bet placement");
                    } else {
                        warn!(
                            "Webhook notification failed with status: {}",
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to call webhook after bet placement: {}", e);
                }
            }
        }
    }
}

#[derive(Debug)]
struct MarketRecord {
    id: String,
    blockchain_market_id: Option<i64>,
    status: String,
}
