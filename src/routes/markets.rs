use axum::{
    extract::{Path, Query, State},
    middleware,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde_json::{json, Value};
use tracing::info;
use utoipa;

use crate::{
    db::Database,
    error::AppError,
    models::MarketQueryParams,
};

use super::protocols::{
    place_bet_alias,
    get_blockchain_status_alias,
    get_blockchain_market,
    create_blockchain_market_alias,
    update_market_image,
    get_market_stats_by_identifier,
};
pub fn create_markets_router() -> Router<Database> {
    
    let public_routes = Router::new()
        .route("/", get(get_markets))
        .route("/stats/platform", get(get_platform_stats))
        .route("/blockchain/status", get(get_blockchain_status_alias))
        .route("/blockchain/:marketId", get(get_blockchain_market))
        .route("/:identifier/stats", get(get_market_stats_by_identifier))
        .route("/:identifier", get(get_market_by_identifier));
    
    
    let protected_routes = Router::new()
        .route("/bet", post(place_bet_alias))
        .route("/create-blockchain", post(create_blockchain_market_alias))
        .route("/:identifier/image", put(update_market_image))
        .layer(middleware::from_fn(crate::middleware::auth::require_api_key));
    
    public_routes.merge(protected_routes)
}

#[utoipa::path(
    get,
    path = "/api/markets",
    tag = "markets",
    params(
        ("limit" = Option<i64>, Query, description = "Number of markets to return (default: 50, max: 100)"),
        ("offset" = Option<i64>, Query, description = "Number of markets to skip for pagination"),
        ("status" = Option<String>, Query, description = "Filter by status: active, resolved, or all"),
    ),
    responses(
        (status = 200, description = "List of markets retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_markets(
    State(db): State<Database>,
    Query(params): Query<MarketQueryParams>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching markets with params: {:?}", params);
    
    let markets = db.get_markets(&params).await?;
    let total = db.count_markets(&params).await?;

    
    let mut market_responses = Vec::new();
    
    for m in markets {
        
        let yield_data = crate::services::yield_calculator::calculate_market_yield_data(
            db.pool(),
            &m.total_pool_size,
            &m.volume,
            &m.end_date,
        ).await.ok();

        let mut market_json = json!({
            "id": m.id,
            "blockchainMarketId": m.blockchain_market_id,
            "marketId": m.market_id,
            "adjTicker": m.adj_ticker,
            "platform": m.platform,
            "question": m.question,
            "description": m.description,
            "rules": m.rules,
            "status": m.status,
            "probability": m.probability,
            "volume": m.volume.to_string(),
            "openInterest": m.open_interest.to_string(),
            "endDate": m.end_date,
            "resolutionDate": m.resolution_date,
            "result": m.result,
            "link": m.link,
            "imageUrl": m.image_url,
            "totalPoolSize": m.total_pool_size.to_string(),
            "yesPoolSize": m.yes_pool_size.to_string(),
            "noPoolSize": m.no_pool_size.to_string(),
            "countYes": m.count_yes,
            "countNo": m.count_no,
            "currentYield": m.current_yield.to_string(),
            "totalYieldEarned": m.total_yield_earned.to_string(),
            "createdAt": m.created_at,
            "updatedAt": m.updated_at,
            "bets": [],
            "_count": { "bets": 0 }
        });

        
        if let Some(yd) = yield_data {
            market_json["dailyYield"] = json!(yd.daily_yield);
            market_json["totalYieldUntilEnd"] = json!(yd.total_yield_until_end);
            market_json["daysRemaining"] = json!(yd.days_remaining);
            market_json["bestProtocolApy"] = json!(yd.best_protocol_apy);
            market_json["bestProtocolName"] = json!(yd.best_protocol_name);
        }

        market_responses.push(market_json);
    }

    Ok(Json(json!({
        "data": market_responses,
        "meta": {
            "total": total,
            "limit": params.limit,
            "offset": params.offset,
            "hasMore": params.offset + params.limit < total
        }
    })))
}


#[utoipa::path(
    get,
    path = "/api/markets/{identifier}",
    tag = "markets",
    params(
        ("identifier" = String, Path, description = "Market identifier (UUID, adjTicker, or blockchain ID)")
    ),
    responses(
        (status = 200, description = "Market details retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_market_by_identifier(
    State(db): State<Database>,
    Path(identifier): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching market by identifier: {}", identifier);

    
    let market = sqlx::query!(
        r#"
        SELECT id, "blockchainMarketId" as blockchain_market_id, "marketId" as market_id, "adjTicker" as adj_ticker,
               platform, question, description, rules, status, probability, volume, "openInterest" as open_interest,
               "endDate" as end_date, "resolutionDate" as resolution_date, result, link, "imageUrl" as image_url,
               "totalPoolSize" as total_pool_size, "yesPoolSize" as yes_pool_size, "noPoolSize" as no_pool_size,
               "countYes" as count_yes, "countNo" as count_no, "currentYield" as current_yield,
               "totalYieldEarned" as total_yield_earned, "createdAt" as created_at, "updatedAt" as updated_at
        FROM markets_extended
        WHERE id = $1 OR "adjTicker" = $1 OR "blockchainMarketId"::text = $1
        LIMIT 1
        "#,
        identifier
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    
    let yield_data = crate::services::yield_calculator::calculate_market_yield_data(
        db.pool(),
        &market.total_pool_size,
        &market.volume,
        &market.end_date,
    ).await.ok();

    let mut market_json = json!({
        "id": market.id,
        "blockchainMarketId": market.blockchain_market_id,
        "marketId": market.market_id,
        "adjTicker": market.adj_ticker,
        "platform": market.platform,
        "question": market.question,
        "description": market.description,
        "rules": market.rules,
        "status": market.status,
        "probability": market.probability,
        "volume": market.volume.to_string(),
        "openInterest": market.open_interest.to_string(),
        "endDate": market.end_date,
        "resolutionDate": market.resolution_date,
        "result": market.result,
        "link": market.link,
        "imageUrl": market.image_url,
        "totalPoolSize": market.total_pool_size.to_string(),
        "yesPoolSize": market.yes_pool_size.to_string(),
        "noPoolSize": market.no_pool_size.to_string(),
        "countYes": market.count_yes,
        "countNo": market.count_no,
        "currentYield": market.current_yield.to_string(),
        "totalYieldEarned": market.total_yield_earned.to_string(),
        "createdAt": market.created_at,
        "updatedAt": market.updated_at,
        "bets": [],
        "_count": { "bets": 0 }
    });

    
    if let Some(yd) = yield_data {
        market_json["dailyYield"] = json!(yd.daily_yield);
        market_json["totalYieldUntilEnd"] = json!(yd.total_yield_until_end);
        market_json["daysRemaining"] = json!(yd.days_remaining);
        market_json["bestProtocolApy"] = json!(yd.best_protocol_apy);
        market_json["bestProtocolName"] = json!(yd.best_protocol_name);
    }

    Ok(Json(json!({
        "data": market_json
    })))
}


#[allow(dead_code)]
async fn get_market_by_id(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let market_id: i64 = id.parse().map_err(|_| AppError::BadRequest("Invalid market ID".to_string()))?;
    
    info!("Fetching market with ID: {}", market_id);

    let market = db
        .get_market_by_id(market_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    let stats = db.get_market_stats(market_id).await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "market": market,
            "stats": stats
        }
    })))
}

#[allow(dead_code)]
async fn get_market_stats(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let market_id: i64 = id.parse().map_err(|_| AppError::BadRequest("Invalid market ID".to_string()))?;
    
    info!("Fetching market stats for ID: {}", market_id);

    let stats = db.get_market_stats(market_id).await?;

    Ok(Json(json!({
        "success": true,
        "data": stats
    })))
}

#[utoipa::path(
    get,
    path = "/api/markets/stats/platform",
    tag = "markets",
    responses(
        (status = 200, description = "Platform statistics retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_platform_stats(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching platform stats");

    let stats = db.get_platform_stats().await?;

    Ok(Json(json!({
        "success": true,
        "data": stats
    })))
}



