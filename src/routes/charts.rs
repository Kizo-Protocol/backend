use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tracing::info;
use utoipa;

use crate::{
    chart::ChartService,
    db::Database,
    error::AppError,
    models::ChartQueryParams,
};
pub fn create_charts_router() -> Router<Database> {
    Router::new()
        .route("/market/:id", get(get_market_chart))
        .route("/market/:id/probability", get(get_market_probability))
        .route("/market/:id/volume", get(get_market_volume))
        .route("/mock/:id", get(get_mock_chart_data))
        .route("/config", get(get_chart_config))
}

#[utoipa::path(
    get,
    path = "/api/charts/market/{id}",
    tag = "charts",
    params(
        ("id" = String, Path, description = "Market ID (numeric or UUID)"),
        ("interval" = Option<String>, Query, description = "Time interval: 1m, 5m, 15m, 1h, 4h, 1d (default: 1h)"),
        ("from" = Option<i64>, Query, description = "Start timestamp"),
        ("to" = Option<i64>, Query, description = "End timestamp"),
        ("series" = Option<String>, Query, description = "Comma-separated series: probability,volume,odds,bets (default: all)")
    ),
    responses(
        (status = 200, description = "Chart data retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_market_chart(
    State(db): State<Database>,
    Path(id): Path<String>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Value>, AppError> {
    
    let market_id: i64 = if let Ok(num_id) = id.parse::<i64>() {
        num_id
    } else {
        
        let result = sqlx::query!(
            r#"SELECT "blockchainMarketId" as blockchain_market_id FROM markets_extended WHERE id = $1 LIMIT 1"#,
            id
        )
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;
        
        result.blockchain_market_id.ok_or_else(|| AppError::BadRequest("Market has no blockchain ID".to_string()))?
    };

    
    if let Some(from) = params.from {
        if from < 0 {
            return Err(AppError::BadRequest("'from' must be a positive number".to_string()));
        }
    }
    if let Some(to) = params.to {
        if to < 0 {
            return Err(AppError::BadRequest("'to' must be a positive number".to_string()));
        }
    }

    info!(
        "Fetching chart data for market {} with interval {}",
        market_id, params.interval
    );

    let chart_service = ChartService::new(db.pool().clone());
    let chart_data = chart_service
        .get_market_chart_data(market_id, &params.interval, params.from, params.to)
        .await?;

    let requested_series: Vec<&str> = params.series.split(',').map(|s| s.trim()).collect();

    let mut response_data = json!({});

    if requested_series.contains(&"probability") {
        response_data["probability"] = json!({
            "yes": chart_data.yes_probability,
            "no": chart_data.no_probability
        });
    }

    if requested_series.contains(&"volume") {
        response_data["volume"] = json!({
            "yes": chart_data.yes_volume,
            "no": chart_data.no_volume,
            "total": chart_data.total_volume
        });
    }

    if requested_series.contains(&"odds") {
        response_data["odds"] = json!({
            "yes": chart_data.yes_odds,
            "no": chart_data.no_odds
        });
    }

    if requested_series.contains(&"bets") {
        response_data["bets"] = serde_json::to_value(&chart_data.bet_count).unwrap();
    }

    Ok(Json(json!({
        "success": true,
        "meta": {
            "symbol": id,
            "interval": params.interval,
            "from": params.from,
            "to": params.to,
            "series": requested_series
        },
        "data": response_data
    })))
}

#[utoipa::path(
    get,
    path = "/api/charts/market/{id}/probability",
    tag = "charts",
    params(
        ("id" = String, Path, description = "Market ID (numeric or UUID)"),
        ("interval" = Option<String>, Query, description = "Time interval (default: 1h)"),
        ("from" = Option<i64>, Query, description = "Start timestamp"),
        ("to" = Option<i64>, Query, description = "End timestamp")
    ),
    responses(
        (status = 200, description = "Probability chart data retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_market_probability(
    State(db): State<Database>,
    Path(id): Path<String>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Value>, AppError> {
    
    let market_id: i64 = if let Ok(num_id) = id.parse::<i64>() {
        num_id
    } else {
        let result = sqlx::query!(
            r#"SELECT "blockchainMarketId" as blockchain_market_id FROM markets_extended WHERE id = $1 LIMIT 1"#,
            id
        )
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;
        
        result.blockchain_market_id.ok_or_else(|| AppError::BadRequest("Market has no blockchain ID".to_string()))?
    };

    let chart_service = ChartService::new(db.pool().clone());
    let chart_data = chart_service
        .get_market_chart_data(market_id, &params.interval, params.from, params.to)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "yes": chart_data.yes_probability,
            "no": chart_data.no_probability
        },
        "meta": {
            "symbol": id,
            "interval": params.interval,
            "data_points": chart_data.yes_probability.len()
        }
    })))
}

#[utoipa::path(
    get,
    path = "/api/charts/market/{id}/volume",
    tag = "charts",
    params(
        ("id" = String, Path, description = "Market ID (numeric or UUID)"),
        ("interval" = Option<String>, Query, description = "Time interval (default: 1h)"),
        ("from" = Option<i64>, Query, description = "Start timestamp"),
        ("to" = Option<i64>, Query, description = "End timestamp")
    ),
    responses(
        (status = 200, description = "Volume chart data retrieved successfully"),
        (status = 404, description = "Market not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_market_volume(
    State(db): State<Database>,
    Path(id): Path<String>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Value>, AppError> {
    
    let market_id: i64 = if let Ok(num_id) = id.parse::<i64>() {
        num_id
    } else {
        let result = sqlx::query!(
            r#"SELECT "blockchainMarketId" as blockchain_market_id FROM markets_extended WHERE id = $1 LIMIT 1"#,
            id
        )
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;
        
        result.blockchain_market_id.ok_or_else(|| AppError::BadRequest("Market has no blockchain ID".to_string()))?
    };

    let chart_service = ChartService::new(db.pool().clone());
    let chart_data = chart_service
        .get_market_chart_data(market_id, &params.interval, params.from, params.to)
        .await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "yes": chart_data.yes_volume,
            "no": chart_data.no_volume,
            "total": chart_data.total_volume
        },
        "meta": {
            "symbol": id,
            "interval": params.interval,
            "data_points": chart_data.total_volume.len()
        }
    })))
}

async fn get_mock_chart_data(
    Path(id): Path<String>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Value>, AppError> {
    info!("Generating mock chart data for market {}", id);

    
    let interval_seconds = match params.interval.as_str() {
        "1m" => 60,
        "5m" => 300,
        "15m" => 900,
        "1h" => 3600,
        "4h" => 14400,
        "1d" => 86400,
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid interval '{}'. Allowed values: 1m, 5m, 15m, 1h, 4h, 1d",
                params.interval
            )))
        }
    };

    let to_timestamp = params.to.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let from_timestamp = params.from.unwrap_or_else(|| to_timestamp - 86400 * 7); 

    let requested_series: Vec<&str> = params.series.split(',').map(|s| s.trim()).collect();

    
    let mut probability_yes = Vec::new();
    let mut probability_no = Vec::new();
    let mut volume_yes = Vec::new();
    let mut volume_no = Vec::new();
    let mut volume_total = Vec::new();
    let mut odds_yes = Vec::new();
    let mut odds_no = Vec::new();
    let mut bet_count = Vec::new();

    let mut current_time = from_timestamp;
    let mut yes_prob: f64 = 0.5;

    while current_time <= to_timestamp {
        
        yes_prob += (rand::random::<f64>() - 0.5) * 0.05;
        yes_prob = yes_prob.clamp(0.1, 0.9);
        let no_prob = 1.0 - yes_prob;

        probability_yes.push(json!({
            "time": current_time,
            "value": yes_prob
        }));
        probability_no.push(json!({
            "time": current_time,
            "value": no_prob
        }));

        
        let yes_vol = (rand::random::<f64>() * 1000.0) as i64;
        let no_vol = (rand::random::<f64>() * 1000.0) as i64;
        volume_yes.push(json!({
            "time": current_time,
            "value": yes_vol
        }));
        volume_no.push(json!({
            "time": current_time,
            "value": no_vol
        }));
        volume_total.push(json!({
            "time": current_time,
            "value": yes_vol + no_vol
        }));

        
        let yes_odd = if yes_prob > 0.0 { 1.0 / yes_prob } else { 2.0 };
        let no_odd = if no_prob > 0.0 { 1.0 / no_prob } else { 2.0 };
        odds_yes.push(json!({
            "time": current_time,
            "value": yes_odd
        }));
        odds_no.push(json!({
            "time": current_time,
            "value": no_odd
        }));

        
        bet_count.push(json!({
            "time": current_time,
            "value": (rand::random::<f64>() * 50.0) as i64
        }));

        current_time += interval_seconds;
    }

    let mut response_data = json!({});

    if requested_series.contains(&"probability") {
        response_data["probability"] = json!({
            "yes": probability_yes,
            "no": probability_no
        });
    }

    if requested_series.contains(&"volume") {
        response_data["volume"] = json!({
            "yes": volume_yes,
            "no": volume_no,
            "total": volume_total
        });
    }

    if requested_series.contains(&"odds") {
        response_data["odds"] = json!({
            "yes": odds_yes,
            "no": odds_no
        });
    }

    if requested_series.contains(&"bets") {
        response_data["bets"] = json!(bet_count);
    }

    Ok(Json(json!({
        "success": true,
        "meta": {
            "symbol": id,
            "interval": params.interval,
            "from": from_timestamp,
            "to": to_timestamp,
            "series": requested_series,
            "mock": true
        },
        "data": response_data
    })))
}

#[utoipa::path(
    get,
    path = "/api/charts/config",
    tag = "charts",
    responses(
        (status = 200, description = "Chart configuration retrieved successfully")
    )
)]
async fn get_chart_config() -> Json<Value> {
    Json(json!({
        "intervals": [
            { "value": "1m", "label": "1 Minute", "seconds": 60 },
            { "value": "5m", "label": "5 Minutes", "seconds": 300 },
            { "value": "15m", "label": "15 Minutes", "seconds": 900 },
            { "value": "1h", "label": "1 Hour", "seconds": 3600 },
            { "value": "4h", "label": "4 Hours", "seconds": 14400 },
            { "value": "1d", "label": "1 Day", "seconds": 86400 }
        ],
        "series": [
            {
                "name": "probability",
                "label": "Probability",
                "type": "line",
                "description": "Yes/No position probabilities over time"
            },
            {
                "name": "volume",
                "label": "Volume",
                "type": "histogram",
                "description": "Betting volume for Yes/No positions"
            },
            {
                "name": "odds",
                "label": "Odds",
                "type": "line",
                "description": "Betting odds for Yes/No positions"
            },
            {
                "name": "bets",
                "label": "Bet Count",
                "type": "histogram",
                "description": "Number of bets placed over time"
            }
        ],
        "colors": {
            "yes": "#22c55e",
            "no": "#ef4444",
            "total": "#6366f1",
            "probability": {
                "yes": "#10b981",
                "no": "#f59e0b"
            }
        },
        "default_timeframe": "7d",
        "max_data_points": 1000
    }))
}



