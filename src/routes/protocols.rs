use axum::{
    extract::{Path, Query, State},
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use tracing::info;
use utoipa;

use crate::{
    db::Database,
    error::AppError,
};
pub fn create_protocols_router() -> Router<Database> {
    
    let public_routes = Router::new()
        .route("/", get(get_protocols))
        .route("/:id", get(get_protocol_by_id))
        .route("/:id/yields", get(get_protocol_yields));
    
    
    let protected_routes = Router::new()
        .route("/:name/apy/update", post(update_protocol_apy))
        .route("/apy/update-all", post(update_all_protocols_apy))
        .layer(middleware::from_fn(crate::middleware::auth::require_api_key));
    
    public_routes.merge(protected_routes)
}

#[utoipa::path(
    get,
    path = "/api/protocols",
    tag = "protocols",
    responses(
        (status = 200, description = "List of protocols retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_protocols(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching all protocols");

    let protocols = sqlx::query_as!(
        crate::models::Protocol,
        r#"
        SELECT id, name, "displayName" as display_name, "baseApy" as base_apy, "isActive" as is_active, description, "iconUrl" as icon_url, "createdAt" as created_at, "updatedAt" as updated_at
        FROM protocols
        WHERE "isActive" = true
        ORDER BY "displayName"
        "#
    )
    .fetch_all(db.pool())
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": protocols,
        "count": protocols.len()
    })))
}

#[utoipa::path(
    get,
    path = "/api/protocols/{id}",
    tag = "protocols",
    params(
        ("id" = String, Path, description = "Protocol ID")
    ),
    responses(
        (status = 200, description = "Protocol details retrieved successfully"),
        (status = 404, description = "Protocol not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_protocol_by_id(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching protocol with ID: {}", id);

    let protocol = sqlx::query_as!(
        crate::models::Protocol,
        r#"
        SELECT id, name, "displayName" as display_name, "baseApy" as base_apy, "isActive" as is_active, description, "iconUrl" as icon_url, "createdAt" as created_at, "updatedAt" as updated_at
        FROM protocols
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Protocol not found".to_string()))?;

    Ok(Json(json!({
        "success": true,
        "data": protocol
    })))
}

async fn get_protocol_yields(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching yields for protocol: {}", id);

    let yields = sqlx::query_as!(
        crate::models::YieldRecord,
        r#"
        SELECT id, "marketId" as market_id, "protocolId" as protocol_id, amount, apy, yield as yield_amount, period, "createdAt" as created_at
        FROM yield_records
        WHERE "protocolId" = $1
        ORDER BY period DESC
        LIMIT 100
        "#,
        id
    )
    .fetch_all(db.pool())
    .await?;

    Ok(Json(json!({
        "success": true,
        "data": yields,
        "count": yields.len()
    })))
}

async fn update_protocol_apy(
    State(db): State<Database>,
    Path(name): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Updating APY from blockchain for protocol: {}", name);

    let yield_service = crate::services::YieldService::new(db.pool().clone());
    
    let updated_apy = yield_service
        .update_protocol_apy_from_blockchain(&name)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update APY: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Successfully updated {} APY from blockchain", name),
        "data": {
            "protocol": name,
            "apy": updated_apy,
            "updated_at": chrono::Utc::now()
        }
    })))
}

async fn update_all_protocols_apy(
    State(db): State<Database>,
) -> Result<Json<Value>, AppError> {
    info!("Updating APY from blockchain for all protocols");

    let yield_service = crate::services::YieldService::new(db.pool().clone());
    
    let results = yield_service
        .update_all_protocols_apy()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update APY: {}", e)))?;

    let protocols: Vec<_> = results.iter().map(|(name, apy)| {
        json!({
            "protocol": name,
            "apy": apy,
        })
    }).collect();

    Ok(Json(json!({
        "success": true,
        "message": format!("Successfully updated {} protocols from blockchain", results.len()),
        "data": protocols,
        "updated_at": chrono::Utc::now()
    })))
}



pub(super) async fn get_market_stats_by_identifier(
    State(db): State<Database>,
    Path(identifier): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching market stats by identifier: {}", identifier);

    
    let market = sqlx::query!(
        r#"SELECT "blockchainMarketId" FROM markets_extended WHERE id = $1 OR "adjTicker" = $1 OR "blockchainMarketId"::text = $1 LIMIT 1"#,
        identifier
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    if let Some(blockchain_id) = market.blockchainMarketId {
        let stats = db.get_market_stats(blockchain_id).await?;
        Ok(Json(json!({
            "success": true,
            "data": stats
        })))
    } else {
        Err(AppError::NotFound("Market not on blockchain".to_string()))
    }
}

pub(super) async fn get_blockchain_market(
    State(db): State<Database>,
    Path(market_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching blockchain market: {}", market_id);

    let market = sqlx::query!(
        r#"
        SELECT id, "blockchainMarketId" as blockchain_market_id, "marketId" as market_id, "adjTicker" as adj_ticker,
               platform, question, description, rules, status, probability, volume, "openInterest" as open_interest,
               "endDate" as end_date, "resolutionDate" as resolution_date, result, link, "imageUrl" as image_url,
               "totalPoolSize" as total_pool_size, "yesPoolSize" as yes_pool_size, "noPoolSize" as no_pool_size,
               "countYes" as count_yes, "countNo" as count_no, "currentYield" as current_yield,
               "totalYieldEarned" as total_yield_earned, "createdAt" as created_at, "updatedAt" as updated_at
        FROM markets_extended
        WHERE "blockchainMarketId" = $1
        "#,
        market_id
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    Ok(Json(json!({
        "data": {
            "id": market.id,
            "blockchainMarketId": market.blockchain_market_id,
            "marketId": market.market_id,
            "adjTicker": market.adj_ticker,
            "platform": market.platform,
            "question": market.question,
            "description": market.description,
            "endDate": market.end_date,
            "status": market.status
        }
    })))
}

pub(super) async fn update_market_image(
    State(db): State<Database>,
    Path(identifier): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    info!("Updating market image for: {}", identifier);

    let image_url = payload["imageUrl"].as_str();

    let result = sqlx::query!(
        r#"UPDATE markets_extended SET "imageUrl" = $1, "updatedAt" = NOW() WHERE id = $1 OR "adjTicker" = $2 RETURNING id, "marketId" as market_id, "adjTicker" as adj_ticker, "imageUrl" as image_url, "updatedAt" as updated_at"#,
        image_url,
        identifier
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    Ok(Json(json!({
        "message": "Market image updated successfully",
        "data": {
            "id": result.id,
            "marketId": result.market_id,
            "adjTicker": result.adj_ticker,
            "imageUrl": result.image_url,
            "updatedAt": result.updated_at
        }
    })))
}


pub(super) async fn place_bet_alias(
    State(db): State<Database>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    
    let bet_request: PlaceBetRequest = serde_json::from_value(payload)
        .map_err(|e| AppError::BadRequest(format!("Invalid bet request: {}", e)))?;
    place_bet(State(db), Json(bet_request)).await
}

pub(super) async fn get_blockchain_status_alias(State(_db): State<Database>) -> Result<Json<Value>, AppError> {
    
    use crate::services::aptos_contract::AptosContractService;
    
    let contract_service = AptosContractService::new()
        .map_err(|e| AppError::Internal(format!("Failed to initialize blockchain service: {}", e)))?;

    let status = contract_service.get_status().await
        .map_err(|e| AppError::Internal(format!("Failed to get blockchain status: {}", e)))?;

    Ok(Json(status))
}

pub(super) async fn create_blockchain_market_alias(
    State(_db): State<Database>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    
    use crate::services::aptos_contract::{AptosContractService, CreateMarketParams};

    let question = payload["question"].as_str()
        .ok_or_else(|| AppError::BadRequest("Question is required".to_string()))?;
    let description = payload["description"].as_str()
        .ok_or_else(|| AppError::BadRequest("Description is required".to_string()))?;
    let duration = payload["duration"].as_u64()
        .ok_or_else(|| AppError::BadRequest("Duration is required".to_string()))?;

    let contract_service = AptosContractService::new()
        .map_err(|e| AppError::Internal(format!("Failed to initialize blockchain service: {}", e)))?;

    
    let protocol_selector_addr = std::env::var("APTOS_PROTOCOL_SELECTOR_ADDR")
        .unwrap_or_else(|_| contract_service.module_address.clone());

    let result = contract_service.create_market(CreateMarketParams {
        question: question.to_string(),
        description: description.to_string(),
        duration_seconds: duration,
        token_type: std::env::var("APTOS_TOKEN_TYPE")
            .unwrap_or_else(|_| "0x1::aptos_coin::AptosCoin".to_string()),
        protocol_selector_addr,
    }).await
        .map_err(|e| AppError::Internal(format!("Failed to create market: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "message": "Market created successfully on blockchain",
        "data": {
            "blockchainMarketId": result.market_id,
            "txHash": result.tx_hash,
            "version": result.version,
            "question": question,
            "description": description,
            "duration": duration
        }
    })))
}



use serde::Deserialize;
use utoipa::{ToSchema, IntoParams};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BetFilters {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(rename = "userAddress")]
    user_address: Option<String>,
    #[serde(rename = "marketId")]
    market_id: Option<String>,
    position: Option<bool>,
}

fn default_limit() -> i64 {
    50
}

#[utoipa::path(
    get,
    path = "/api/bets",
    tag = "bets",
    params(BetFilters),
    responses(
        (status = 200, description = "List of bets retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub(super) async fn get_bets_with_filters(
    State(_db): State<Database>,
    Query(filters): Query<BetFilters>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching bets with filters: {:?}", filters);

    
    let mut query_str = "SELECT * FROM bets".to_string();
    let mut count_str = "SELECT COUNT(*) FROM bets".to_string();
    let mut where_parts = Vec::new();

    
    if filters.user_address.is_some() {
        where_parts.push("user_addr = $1".to_string());
    }
    
    let mut next_param = if filters.user_address.is_some() { 2 } else { 1 };
    if filters.market_id.is_some() {
        where_parts.push(format!("market_id = ${}", next_param));
        next_param += 1;
    }
    
    if filters.position.is_some() {
        where_parts.push(format!("position = ${}", next_param));
        next_param += 1;
    }

    
    if !where_parts.is_empty() {
        let where_clause = format!(" WHERE {}", where_parts.join(" AND "));
        query_str.push_str(&where_clause);
        count_str.push_str(&where_clause);
    }

    
    query_str.push_str(&format!(" ORDER BY inserted_at DESC LIMIT ${} OFFSET ${}", next_param, next_param + 1));

    
    let mut main_query = sqlx::query_as::<_, crate::models::Bet>(&query_str);
    if let Some(ref addr) = filters.user_address {
        main_query = main_query.bind(addr);
    }
    if let Some(ref market) = filters.market_id {
        if let Ok(market_num) = market.parse::<i64>() {
            main_query = main_query.bind(market_num);
        } else {
            
            return Ok(Json(json!({
                "data": Vec::<crate::models::Bet>::new(),
                "meta": {
                    "total": 0,
                    "limit": filters.limit,
                    "offset": filters.offset,
                    "hasMore": false
                }
            })));
        }
    }
    if let Some(pos) = filters.position {
        main_query = main_query.bind(pos);
    }
    main_query = main_query.bind(filters.limit).bind(filters.offset);

    let bets = main_query.fetch_all(_db.pool()).await?;

    
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_str);
    if let Some(ref addr) = filters.user_address {
        count_query = count_query.bind(addr);
    }
    if let Some(ref market) = filters.market_id {
        if let Ok(market_num) = market.parse::<i64>() {
            count_query = count_query.bind(market_num);
        }
    }
    if let Some(pos) = filters.position {
        count_query = count_query.bind(pos);
    }

    let total: i64 = count_query.fetch_one(_db.pool()).await.unwrap_or(0);

    Ok(Json(json!({
        "data": bets,
        "meta": {
            "total": total,
            "limit": filters.limit,
            "offset": filters.offset,
            "hasMore": filters.offset + filters.limit < total
        }
    })))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PlaceBetRequest {
    #[serde(rename = "marketIdentifier")]
    market_identifier: String,
    position: bool,
    amount: String,
    #[serde(rename = "userAddress")]
    user_address: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ClaimWinningsRequest {
    #[serde(rename = "marketIdentifier")]
    market_identifier: String,
    #[serde(rename = "userAddress")]
    user_address: String,
    #[serde(rename = "betIndex")]
    bet_index: u64,
}

#[utoipa::path(
    post,
    path = "/api/bets",
    tag = "bets",
    request_body = PlaceBetRequest,
    responses(
        (status = 201, description = "Bet placed successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub(super) async fn place_bet(
    State(db): State<Database>,
    Json(payload): Json<PlaceBetRequest>,
) -> Result<Json<Value>, AppError> {
    info!("Placing bet on market: {}", payload.market_identifier);

    
    let betting_service = crate::services::betting_service::BettingService::new(db.pool().clone())
        .map_err(|e| AppError::Internal(format!("Failed to initialize betting service: {}", e)))?;

    
    let params = crate::services::betting_service::PlaceBetParams {
        market_identifier: payload.market_identifier,
        user_address: payload.user_address,
        position: payload.position,
        amount: payload.amount,
    };

    
    let result = betting_service.place_bet(params).await
        .map_err(|e| AppError::Internal(format!("Failed to place bet: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "message": "Bet placed successfully",
        "data": {
            "betId": result.bet_id,
            "blockchainBetId": result.blockchain_bet_id,
            "marketId": result.market_id,
            "blockchainMarketId": result.blockchain_market_id,
            "position": result.position,
            "amount": result.amount,
            "txHash": result.tx_hash,
            "user": {
                "address": result.user_address
            },
            "blockchain": {
                "txHash": result.tx_hash,
                "betId": result.blockchain_bet_id
            },
            "explorer": {
                "transaction": format!("https://explorer.aptoslabs.com/txn/{}?network=testnet", result.tx_hash)
            }
        }
    })))
}

pub(super) async fn claim_winnings_route(
    State(db): State<Database>,
    Json(payload): Json<ClaimWinningsRequest>,
) -> Result<Json<Value>, AppError> {
    info!("Claiming winnings for market: {}", payload.market_identifier);

    
    let betting_service = crate::services::betting_service::BettingService::new(db.pool().clone())
        .map_err(|e| AppError::Internal(format!("Failed to initialize betting service: {}", e)))?;

    
    let params = crate::services::betting_service::ClaimWinningsParams {
        market_identifier: payload.market_identifier,
        user_address: payload.user_address,
        bet_index: payload.bet_index,
    };

    
    let result = betting_service.claim_winnings(params).await
        .map_err(|e| AppError::Internal(format!("Failed to claim winnings: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "message": "Winnings claimed successfully",
        "data": {
            "betId": result.bet_id,
            "winningAmount": result.winning_amount,
            "yieldShare": result.yield_share,
            "totalClaimed": result.total_claimed,
            "txHash": result.tx_hash,
            "explorer": {
                "transaction": format!("https://explorer.aptoslabs.com/txn/{}?network=testnet", result.tx_hash)
            }
        }
    })))
}

#[derive(Debug, Deserialize)]
pub(super) struct WebhookSyncRequest {
    #[serde(rename = "syncType")]
    sync_type: Option<String>,  
    #[serde(rename = "marketId")]
    #[allow(dead_code)]  
    market_id: Option<i64>,
    #[serde(rename = "userId")]
    #[allow(dead_code)]  
    user_id: Option<String>,
}


pub(super) async fn webhook_sync_data(
    State(db): State<Database>,
    Json(payload): Json<WebhookSyncRequest>,
) -> Result<Json<Value>, AppError> {
    info!("Webhook triggered for data sync: {:?}", payload);

    let sync_service = crate::services::blockchain_sync::BlockchainSyncService::new(db.pool().clone());
    
    let sync_type = payload.sync_type.as_deref().unwrap_or("full");
    
    match sync_type {
        "bet" => {
            
            let result = sync_service.sync_bets().await
                .map_err(|e| AppError::Internal(format!("Bet sync failed: {}", e)))?;
            
            
            sync_service.update_market_stats().await
                .map_err(|e| AppError::Internal(format!("Stats update failed: {}", e)))?;
                
            Ok(Json(json!({
                "success": true,
                "message": "Bet data synchronized successfully",
                "data": {
                    "syncType": "bet",
                    "processed": result.processed,
                    "newEvents": result.new_events,
                    "errors": result.errors
                }
            })))
        },
        "market" => {
            
            let result = sync_service.sync_markets().await
                .map_err(|e| AppError::Internal(format!("Market sync failed: {}", e)))?;
            
            Ok(Json(json!({
                "success": true,
                "message": "Market data synchronized successfully",
                "data": {
                    "syncType": "market",
                    "processed": result.processed,
                    "newEvents": result.new_events,
                    "errors": result.errors
                }
            })))
        },
        _ => {
            
            let result = sync_service.run_full_sync().await
                .map_err(|e| AppError::Internal(format!("Full sync failed: {}", e)))?;
            
            Ok(Json(json!({
                "success": true,
                "message": "Full data synchronization completed",
                "data": {
                    "syncType": "full",
                    "totalProcessed": result.total_processed,
                    "totalErrors": result.total_errors,
                    "durationMs": result.duration_ms,
                    "results": result.results.len()
                }
            })))
        }
    }
}

pub(super) async fn get_bet_stats_summary(
    State(_db): State<Database>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let user_address = params.get("userAddress");
    
    info!("Fetching bet stats summary for user: {:?}", user_address);

    if let Some(address) = user_address {
        
        let total_bets = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM bets_extended WHERE "userId" = $1"#
        )
        .bind(address)
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0);

        let active_bets = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM bets_extended WHERE "userId" = $1 AND status = 'active'"#
        )
        .bind(address)
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0);

        let won_bets = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM bets_extended WHERE "userId" = $1 AND status = 'won'"#
        )
        .bind(address)
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0);

        let lost_bets = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM bets_extended WHERE "userId" = $1 AND status = 'lost'"#
        )
        .bind(address)
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0);

        let total_amount = sqlx::query_scalar::<_, Option<sqlx::types::BigDecimal>>(
            r#"SELECT SUM(amount) FROM bets_extended WHERE "userId" = $1"#
        )
        .bind(address)
        .fetch_one(_db.pool())
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| sqlx::types::BigDecimal::from(0));

        let total_payout = sqlx::query_scalar::<_, Option<sqlx::types::BigDecimal>>(
            r#"SELECT SUM(payout) FROM bets_extended WHERE "userId" = $1 AND status = 'won'"#
        )
        .bind(address)
        .fetch_one(_db.pool())
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| sqlx::types::BigDecimal::from(0));

        let win_rate = if total_bets > 0 {
            (won_bets as f64 / total_bets as f64) * 100.0
        } else {
            0.0
        };

        let profit = &total_payout - &total_amount;

        Ok(Json(json!({
            "data": {
                "totalBets": total_bets,
                "activeBets": active_bets,
                "wonBets": won_bets,
                "lostBets": lost_bets,
                "winRate": win_rate,
                "totalAmount": total_amount.to_string(),
                "totalPayout": total_payout.to_string(),
                "profit": profit.to_string()
            }
        })))
    } else {
        
        let total_bets = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM bets_extended"#
        )
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0);

        let active_bets = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM bets_extended WHERE status = 'active'"#
        )
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0);

        Ok(Json(json!({
            "data": {
                "totalBets": total_bets,
                "activeBets": active_bets,
                "wonBets": 0,
                "lostBets": 0,
                "winRate": 0.0,
                "totalAmount": "0",
                "totalPayout": "0",
                "profit": "0"
            }
        })))
    }
}



