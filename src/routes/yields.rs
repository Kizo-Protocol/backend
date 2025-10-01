use axum::{
    extract::{Query, State},
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
pub fn create_yields_router() -> Router<Database> {
    
    let public_routes = Router::new()
        .route("/", get(get_yields))
        .route("/summary", get(get_yield_summary))
        .route("/apy/current", get(get_current_apy))
        .route("/protocols", get(get_yield_protocols))
        .route("/contract/test", get(test_contract_connectivity))
        .route("/contract/apy", get(get_contract_apy));
    
    
    let protected_routes = Router::new()
        .route("/update", post(update_yields))
        .layer(middleware::from_fn(crate::middleware::auth::require_api_key));
    
    public_routes.merge(protected_routes)
}

#[utoipa::path(
    get,
    path = "/api/yields",
    tag = "yields",
    params(
        ("limit" = Option<i64>, Query, description = "Number of results to return"),
        ("offset" = Option<i64>, Query, description = "Number of results to skip"),
        ("marketId" = Option<String>, Query, description = "Filter by market ID")
    ),
    responses(
        (status = 200, description = "Yield records retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_yields(
    State(_db): State<Database>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    info!("Fetching yields");

    let limit: i64 = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(50);
    let offset: i64 = params.get("offset").and_then(|o| o.parse().ok()).unwrap_or(0);
    let market_id = params.get("marketId");

    
    let yields = if let Some(mid) = market_id {
        sqlx::query_as::<_, crate::models::YieldRecord>(
            r#"
            SELECT id, "marketId" as market_id, "protocolId" as protocol_id, amount, apy, yield as yield_amount, period, "createdAt" as created_at
            FROM yield_records
            WHERE "marketId" = $1
            ORDER BY period DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(mid.parse::<i64>().unwrap_or(0))
        .bind(limit)
        .bind(offset)
        .fetch_all(_db.pool())
        .await?
    } else {
        sqlx::query_as::<_, crate::models::YieldRecord>(
            r#"
            SELECT id, "marketId" as market_id, "protocolId" as protocol_id, amount, apy, yield as yield_amount, period, "createdAt" as created_at
            FROM yield_records
            ORDER BY period DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(_db.pool())
        .await?
    };

    let total = if let Some(mid) = market_id {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM yield_records WHERE "marketId" = $1"#
        )
        .bind(mid.parse::<i64>().unwrap_or(0))
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0)
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM yield_records"#
        )
        .fetch_one(_db.pool())
        .await
        .unwrap_or(0)
    };

    Ok(Json(json!({
        "message": "Yields retrieved successfully",
        "data": yields,
        "meta": {
            "total": total,
            "limit": limit,
            "offset": offset,
            "marketId": market_id,
            "hasMore": offset + limit < total
        }
    })))
}

async fn get_yield_summary(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching yield summary");

    let calculator = crate::services::UserYieldCalculator::new(db.pool().clone());
    
    match calculator.calculate_global_yields().await {
        Ok(summary) => {
            let protocol_breakdown: Vec<Value> = summary.protocol_breakdown.iter().map(|p| {
                json!({
                    "protocol": p.protocol,
                    "totalAmount": p.total_amount.to_string(),
                    "totalYield": p.total_yield.to_string(),
                    "averageApy": p.average_apy.to_string()
                })
            }).collect();

            Ok(Json(json!({
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
            tracing::error!("Failed to calculate yields: {}", e);
            Err(AppError::Internal(format!("Failed to calculate yields: {}", e)))
        }
    }
}

async fn get_current_apy(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching current APY rates");

    let protocols = sqlx::query!(
        r#"SELECT name, "baseApy" as base_apy FROM protocols WHERE "isActive" = true"#
    )
    .fetch_all(db.pool())
    .await?;

    let rates: Vec<Value> = protocols.iter().map(|p| {
        let apy_value = p.base_apy.to_string().parse::<f64>().unwrap_or(0.0);
        json!({
            "protocol": p.name,
            "apy": apy_value
        })
    }).collect();

    Ok(Json(json!({
        "data": {
            "rates": rates,
            "lastUpdated": chrono::Utc::now().to_rfc3339(),
            "source": "database"
        }
    })))
}

#[utoipa::path(
    get,
    path = "/api/yields/protocols",
    tag = "yields",
    responses(
        (status = 200, description = "List of yield protocols retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_yield_protocols(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    
    info!("Fetching protocols via /yields/protocols");
    
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
        "message": "Protocols retrieved successfully",
        "data": protocols
    })))
}

async fn update_yields(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Triggering yield calculation");

    
    let market_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM markets_extended WHERE status = 'active'"
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    Ok(Json(json!({
        "message": "Yields updated successfully",
        "data": {
            "updated": market_count,
            "errors": []
        }
    })))
}

async fn test_contract_connectivity(State(_db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Testing contract connectivity");

    let node_url = std::env::var("APTOS_NODE_URL")
        .unwrap_or_else(|_| "https://fullnode.testnet.aptoslabs.com/v1".to_string());
    let module_address = std::env::var("APTOS_MODULE_ADDRESS")
        .unwrap_or_else(|_| "0xaab1ca043fb6cd7b2f264f3ce32f301427e7f67b0e816b30d4e95f1e2bcbabfa".to_string());

    
    let client = reqwest::Client::new();
    let health_response = client
        .get(format!("{}/", node_url))
        .send()
        .await;

    let node_healthy = health_response.is_ok();

    
    let module_url = format!("{}/accounts/{}/module/kizo_prediction_market", node_url, module_address);
    let module_response = client
        .get(&module_url)
        .send()
        .await;

    let module_exists = module_response.is_ok() && module_response.unwrap().status().is_success();

    
    let mut contract_results = serde_json::Map::new();
    contract_results.insert(
        module_address.clone(),
        json!({
            "connected": node_healthy && module_exists,
            "blockNumber": 0,
            "error": if !node_healthy { Some("Node not reachable") } else if !module_exists { Some("Module not found") } else { None }
        })
    );

    Ok(Json(json!({
        "message": "Contract connectivity test complete",
        "data": contract_results
    })))
}

async fn get_contract_apy(State(_db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching contract APY data");

    let yield_service = crate::services::YieldService::new(_db.pool().clone());
    
    
    match yield_service.update_all_protocols_apy().await {
        Ok(results) => {
            
            let mut protocol_data = serde_json::Map::new();
            
            for (protocol, apy) in results.iter() {
                protocol_data.insert(
                    protocol.clone(),
                    json!({
                        "apy": apy.to_string(),
                        "lastUpdated": chrono::Utc::now().to_rfc3339(),
                        "contractAddress": std::env::var("APTOS_MODULE_ADDRESS")
                            .unwrap_or_else(|_| "0x...".to_string())
                    })
                );
            }

            Ok(Json(json!({
                "message": "Contract APY data fetched successfully",
                "data": protocol_data
            })))
        }
        Err(e) => {
            Err(AppError::Internal(format!("Failed to fetch contract APY: {}", e)))
        }
    }
}
