use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{
    db::Database,
    error::AppError,
    services::aptos_contract::{AptosContractService, CreateMarketParams},
};

pub fn create_blockchain_router(db: Database) -> Router<Database> {
    Router::new()
        .route("/create-aptos", post(create_market_on_aptos))
        .route("/status", get(get_blockchain_status))
        .with_state(db)
}

#[derive(Debug, Deserialize)]
struct CreateAptosMarketRequest {
    db_market_id: Option<String>,
    question: String,
    description: String,
    duration: u64,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct CreateAptosMarketResponse {
    success: bool,
    message: String,
    data: Value,
}

async fn create_market_on_aptos(
    State(db): State<Database>,
    Json(payload): Json<CreateAptosMarketRequest>,
) -> Result<Json<Value>, AppError> {
    info!(
        "Creating market on Aptos blockchain: {}",
        payload.question
    );

    
    if payload.question.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Question is required".to_string(),
        ));
    }

    if payload.duration < 3600 {
        return Err(AppError::BadRequest(
            "Duration must be at least 1 hour (3600 seconds)".to_string(),
        ));
    }

    
    let contract_service = AptosContractService::new().map_err(|e| {
        error!("Failed to initialize Aptos contract service: {}", e);
        AppError::Internal(format!(
            "Failed to initialize blockchain service: {}",
            e
        ))
    })?;

    
    let protocol_selector_addr = std::env::var("APTOS_PROTOCOL_SELECTOR_ADDR")
        .unwrap_or_else(|_| contract_service.module_address.clone());

    
    let result = contract_service
        .create_market(CreateMarketParams {
            question: payload.question.clone(),
            description: payload.description.clone(),
            duration_seconds: payload.duration,
            token_type: std::env::var("APTOS_TOKEN_TYPE")
                .unwrap_or_else(|_| "0x1::aptos_coin::AptosCoin".to_string()),
            protocol_selector_addr,
        })
        .await
        .map_err(|e| {
            error!("Failed to create market on Aptos: {}", e);
            AppError::Internal(format!(
                "Failed to create market on blockchain: {}",
                e
            ))
        })?;

    info!(
        "Market created on Aptos! Blockchain ID: {}, TX: {}",
        result.market_id, result.tx_hash
    );

    
    if let Some(ref db_id) = payload.db_market_id {
        let update_result = sqlx::query(
            "UPDATE markets_extended 
             SET \"blockchainMarketId\" = $1, \"updatedAt\" = NOW() 
             WHERE id = $2",
        )
        .bind(result.market_id)
        .bind(db_id)
        .execute(db.pool())
        .await;

        match update_result {
            Ok(_) => {
                info!(
                    "Updated market {} with blockchain ID {}",
                    db_id, result.market_id
                );
            }
            Err(e) => {
                error!("Failed to update market with blockchain ID: {}", e);
                
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "Market created successfully on Aptos blockchain",
        "data": {
            "blockchain_market_id": result.market_id,
            "tx_hash": result.tx_hash,
            "version": result.version,
            "question": payload.question,
            "description": payload.description,
            "duration": payload.duration,
            "explorer": format!("https://explorer.aptoslabs.com/txn/{}?network=testnet", result.tx_hash),
            "db_market_id": payload.db_market_id
        }
    })))
}

async fn get_blockchain_status() -> Result<Json<Value>, AppError> {
    let contract_service = AptosContractService::new().map_err(|e| {
        error!("Failed to initialize Aptos contract service: {}", e);
        AppError::Internal(format!(
            "Failed to initialize blockchain service: {}",
            e
        ))
    })?;

    let status = contract_service.get_status().await.map_err(|e| {
        error!("Failed to get blockchain status: {}", e);
        AppError::Internal(format!(
            "Failed to connect to blockchain: {}",
            e
        ))
    })?;

    Ok(Json(status))
}