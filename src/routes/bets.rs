use axum::{
    extract::{Path, Query, State},
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use tracing::info;

use crate::{db::Database, error::AppError, models::PaginationParams};

use super::protocols::{
    claim_winnings_route, get_bet_stats_summary, get_bets_with_filters, place_bet,
};
pub fn create_bets_router() -> Router<Database> {
    let public_routes = Router::new()
        .route("/", get(get_bets_with_filters))
        .route("/stats/summary", get(get_bet_stats_summary))
        .route("/:id", get(get_bet_by_id))
        .route("/user/:address", get(get_user_bets))
        .route("/user/:address/stats", get(get_user_stats))
        .route("/user/:address/yields", get(get_user_yields))
        .route("/market/:market_id", get(get_market_bets));

    let protected_routes = Router::new()
        .route("/", post(place_bet))
        .route("/claim", post(claim_winnings_route))
        .layer(middleware::from_fn(
            crate::middleware::auth::require_api_key,
        ));

    public_routes.merge(protected_routes)
}

async fn get_bet_by_id(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let bet_id: i64 = id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid bet ID".to_string()))?;

    info!("Fetching bet with ID: {}", bet_id);

    let bet = db
        .get_bet_by_id(bet_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;

    Ok(Json(json!({
        "success": true,
        "data": bet
    })))
}

async fn get_user_bets(
    State(db): State<Database>,
    Path(address): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching bets for user: {}", address);

    let bets = db.get_bets_by_user(&address, &params).await?;

    Ok(Json(json!({
        "success": true,
        "data": bets,
        "count": bets.len(),
        "limit": params.limit,
        "offset": params.offset
    })))
}

async fn get_user_stats(
    State(db): State<Database>,
    Path(address): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching stats for user: {}", address);

    let stats = db.get_user_stats(&address).await?;

    Ok(Json(json!({
        "success": true,
        "data": stats
    })))
}

async fn get_user_yields(
    State(db): State<Database>,
    Path(address): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching yields for user: {}", address);

    let calculator = crate::services::UserYieldCalculator::new(db.pool().clone());

    match calculator.calculate_user_yields(&address).await {
        Ok(summary) => {
            let protocol_breakdown: Vec<Value> = summary
                .protocol_breakdown
                .iter()
                .map(|p| {
                    json!({
                        "protocol": p.protocol,
                        "totalAmount": p.total_amount.to_string(),
                        "totalYield": p.total_yield.to_string(),
                        "averageApy": p.average_apy.to_string()
                    })
                })
                .collect();

            Ok(Json(json!({
                "success": true,
                "data": {
                    "protocolBreakdown": protocol_breakdown,
                    "totals": {
                        "totalYield": summary.total_yield_earned.to_string(),
                        "totalAmount": summary.total_amount_staked.to_string(),
                        "averageApy": summary.average_apy.to_string(),
                        "activePoolSize": summary.active_pool_size.to_string()
                    },
                    "recentPerformance": []
                }
            })))
        }
        Err(e) => {
            tracing::error!("Failed to calculate user yields: {}", e);
            Err(AppError::Internal(format!(
                "Failed to calculate user yields: {}",
                e
            )))
        }
    }
}

async fn get_market_bets(
    State(db): State<Database>,
    Path(market_id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    let market_id: i64 = market_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid market ID".to_string()))?;

    info!("Fetching bets for market: {}", market_id);

    let bets = db.get_bets_by_market(market_id, &params).await?;

    Ok(Json(json!({
        "success": true,
        "data": bets,
        "count": bets.len(),
        "limit": params.limit,
        "offset": params.offset
    })))
}
