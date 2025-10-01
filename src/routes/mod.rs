use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use utoipa;

use crate::db::Database;
use crate::error::AppError;


mod auth;
mod blockchain;
pub mod markets;
pub mod bets;
pub mod charts;
pub mod sync;
pub mod protocols;
pub mod yields;
pub mod prices;


pub fn create_router(db: Database) -> Router {
    Router::new()
        .route("/", get(api_info))
        .route("/health", get(health_check))
        .nest("/auth", auth::create_auth_router())
        .nest("/markets", markets::create_markets_router())
        .nest("/bets", bets::create_bets_router())
        .nest("/charts", charts::create_charts_router())
        .nest("/sync", sync::create_sync_router())
        .nest("/protocols", protocols::create_protocols_router())
        .nest("/yields", yields::create_yields_router())
        .nest("/prices", prices::create_prices_router())
        .merge(blockchain::create_blockchain_router(db.clone()))
        .with_state(db)
}


async fn api_info() -> Result<Json<Value>, AppError> {
    Ok(Json(json!({
        "name": "Kizo Prediction Markets API",
        "version": "1.0.0",
        "status": "operational"
    })))
}


#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy"),
        (status = 503, description = "Service is unhealthy")
    )
)]
async fn health_check(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    
    let result = sqlx::query!("SELECT 1 as check")
        .fetch_one(db.pool())
        .await;

    match result {
        Ok(_) => Ok(Json(json!({
            "status": "healthy",
            "database": "connected",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))),
        Err(e) => Ok(Json(json!({
            "status": "unhealthy",
            "database": "disconnected",
            "error": e.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }
}
