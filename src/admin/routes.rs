use axum::{
    extract::{Query, State},
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{
    db::Database,
    error::AppError,
    services::{
        aptos_contract::{AptosContractService, CreateMarketParams},
        market_seeder::MarketSeeder,
    },
};

pub fn create_admin_router(db: Database) -> Router {
    Router::new()
        .route("/seed-markets", post(seed_markets))
        .route(
            "/sync-markets-to-blockchain",
            post(sync_markets_to_blockchain),
        )
        .with_state(db)
}

#[derive(Debug, Deserialize)]
pub struct SeedMarketsQuery {
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    10
}

async fn seed_markets(
    State(db): State<Database>,
    Query(params): Query<SeedMarketsQuery>,
) -> Result<Json<Value>, AppError> {
    info!("Admin: Seed markets requested, count: {}", params.count);

    let api_key = std::env::var("ADJACENT_API_KEY")
        .map_err(|_| AppError::Internal("ADJACENT_API_KEY not configured".to_string()))?;

    let seeder = MarketSeeder::new(db.pool().clone(), api_key)
        .map_err(|e| AppError::Internal(format!("Failed to create seeder: {}", e)))?;

    let result = seeder
        .seed_markets(params.count)
        .await
        .map_err(|e| AppError::Internal(format!("Seeding failed: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "total_requested": result.total_requested,
            "fetched_from_api": result.fetched_from_api,
            "created": result.created,
            "updated": result.updated,
            "skipped": result.skipped,
            "errors": result.errors
        },
        "message": format!(
            "Seeded {} markets: {} created, {} updated",
            result.fetched_from_api, result.created, result.updated
        )
    })))
}

#[derive(Debug, Serialize)]
pub struct SeedMarketsResponse {
    pub success: bool,
    pub data: SeedMarketsData,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SeedMarketsData {
    pub total_requested: usize,
    pub fetched_from_api: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[derive(Debug, Deserialize)]
pub struct SyncMarketsQuery {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SyncMarketsResponse {
    pub success: bool,
    pub data: SyncMarketsData,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SyncMarketsData {
    pub total_found: usize,
    pub synced: usize,
    pub failed: usize,
    pub markets: Vec<SyncedMarket>,
}

#[derive(Debug, Serialize)]
pub struct SyncedMarket {
    pub market_id: String,
    pub question: String,
    pub blockchain_market_id: Option<i64>,
    pub status: String,
}

async fn sync_markets_to_blockchain(
    State(db): State<Database>,
    Query(params): Query<SyncMarketsQuery>,
) -> Result<Json<Value>, AppError> {
    info!("Admin: Sync markets to blockchain requested");

    let aptos_service = AptosContractService::new()
        .map_err(|e| AppError::Internal(format!("Failed to initialize Aptos service: {}", e)))?;

    let module_addr = std::env::var("APTOS_MODULE_ADDRESS")
        .map_err(|_| AppError::Internal("APTOS_MODULE_ADDRESS not configured".to_string()))?;
    let protocol_selector_addr =
        std::env::var("APTOS_PROTOCOL_SELECTOR_ADDR").unwrap_or_else(|_| module_addr.clone());

    let limit = params.limit.unwrap_or(10);
    let markets = sqlx::query!(
        r#"
        SELECT id, "marketId", question, description, "endDate"
        FROM markets_extended
        WHERE "blockchainMarketId" IS NULL
        AND status = 'active'
        ORDER BY "createdAt" DESC
        LIMIT $1
        "#,
        limit as i64
    )
    .fetch_all(db.pool())
    .await?;

    let total_found = markets.len();
    info!("Found {} markets without blockchain ID", total_found);

    let mut synced = 0;
    let mut failed = 0;
    let mut synced_markets = Vec::new();

    for market in markets {
        let market_id_str = market.marketId.clone().unwrap_or_default();
        info!(
            "Processing market: {} - {}",
            market_id_str,
            market.question.as_deref().unwrap_or("No question")
        );

        let now = chrono::Utc::now();
        let end_utc = market.endDate.and_utc();
        let duration = end_utc.signed_duration_since(now);
        let duration_seconds = duration.num_seconds().max(0) as u64;

        let params = CreateMarketParams {
            question: market
                .question
                .clone()
                .unwrap_or_else(|| "Untitled Market".to_string()),
            description: market
                .description
                .clone()
                .unwrap_or_else(|| "No description".to_string()),
            duration_seconds,
            token_type: "0x1::aptos_coin::AptosCoin".to_string(),
            protocol_selector_addr: protocol_selector_addr.clone(),
        };

        match aptos_service.create_market(params).await {
            Ok(result) => {
                info!(
                    "Created market on blockchain: ID {}, TX: {}",
                    result.market_id, result.tx_hash
                );

                match sqlx::query!(
                    r#"UPDATE markets_extended SET "blockchainMarketId" = $1 WHERE id = $2"#,
                    result.market_id,
                    market.id
                )
                .execute(db.pool())
                .await
                {
                    Ok(_) => {
                        synced += 1;
                        synced_markets.push(SyncedMarket {
                            market_id: market_id_str,
                            question: market.question.unwrap_or_default(),
                            blockchain_market_id: Some(result.market_id),
                            status: "synced".to_string(),
                        });
                    }
                    Err(e) => {
                        error!("Failed to update database for market {}: {}", market.id, e);
                        failed += 1;
                        synced_markets.push(SyncedMarket {
                            market_id: market_id_str,
                            question: market.question.unwrap_or_default(),
                            blockchain_market_id: Some(result.market_id),
                            status: format!("blockchain_created_but_db_update_failed: {}", e),
                        });
                    }
                }
            }
            Err(e) => {
                error!("Failed to create market on blockchain: {}", e);
                failed += 1;
                synced_markets.push(SyncedMarket {
                    market_id: market_id_str,
                    question: market.question.unwrap_or_default(),
                    blockchain_market_id: None,
                    status: format!("failed: {}", e),
                });
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "total_found": total_found,
            "synced": synced,
            "failed": failed,
            "markets": synced_markets
        },
        "message": format!(
            "Sync complete: {}/{} markets synced to blockchain",
            synced, total_found
        )
    })))
}
