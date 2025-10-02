use axum::{extract::State, response::Json, routing::get, Router};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::db::Database;
use crate::error::AppError;
use crate::services::chainlink_price_feed::ChainlinkPriceFeed;

pub fn create_prices_router() -> Router<Database> {
    Router::new()
        .route("/apt-usd", get(get_apt_usd_price))
        .route("/apt-usd/refresh", get(refresh_apt_usd_price))
}

#[utoipa::path(
    get,
    path = "/api/prices/apt-usd",
    tag = "prices",
    responses(
        (status = 200, description = "Successfully retrieved APT/USD price", body = Value),
        (status = 500, description = "Failed to fetch price")
    )
)]
async fn get_apt_usd_price(State(_db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching APT/USD price from Chainlink");

    let price_feed = ChainlinkPriceFeed::new().map_err(|e| {
        error!("Failed to initialize price feed: {}", e);
        AppError::InternalError(format!("Price feed initialization failed: {}", e))
    })?;

    match price_feed.get_apt_usd_price().await {
        Ok(price) => {
            let cached_data = price_feed.get_cached_price().await;

            Ok(Json(json!({
                "success": true,
                "data": {
                    "price": price,
                    "symbol": "APT/USD",
                    "source": "Multi-Exchange Aggregator",
                    "source_count": cached_data.as_ref().map(|d| d.round_id).unwrap_or(0),
                    "decimals": cached_data.as_ref().map(|d| d.decimals).unwrap_or(8),
                    "timestamp": cached_data.as_ref().map(|d| d.timestamp).unwrap_or(chrono::Utc::now().timestamp()),
                    "aggregation_method": "median",
                },
                "formatted": format!("${:.2}", price)
            })))
        }
        Err(e) => {
            error!("Failed to fetch APT/USD price: {}", e);
            Err(AppError::InternalError(format!(
                "Failed to fetch price: {}",
                e
            )))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/prices/apt-usd/refresh",
    tag = "prices",
    responses(
        (status = 200, description = "Successfully refreshed APT/USD price", body = Value),
        (status = 500, description = "Failed to refresh price")
    )
)]
async fn refresh_apt_usd_price(State(_db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Force refreshing APT/USD price from Chainlink");

    let price_feed = ChainlinkPriceFeed::new().map_err(|e| {
        error!("Failed to initialize price feed: {}", e);
        AppError::InternalError(format!("Price feed initialization failed: {}", e))
    })?;

    match price_feed.refresh_price().await {
        Ok(price_data) => Ok(Json(json!({
            "success": true,
            "data": {
                "price": price_data.price,
                "symbol": "APT/USD",
                "source": "Multi-Exchange Aggregator",
                "source_count": price_data.round_id,
                "decimals": price_data.decimals,
                "timestamp": price_data.timestamp,
                "aggregation_method": "median",
            },
            "formatted": format!("${:.2}", price_data.price),
            "message": "Price refreshed successfully"
        }))),
        Err(e) => {
            error!("Failed to refresh APT/USD price: {}", e);
            Err(AppError::InternalError(format!(
                "Failed to refresh price: {}",
                e
            )))
        }
    }
}
