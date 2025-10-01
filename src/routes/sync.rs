use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use tracing::info;

use crate::{
    db::Database,
    error::AppError,
    services::scheduler::Scheduler,
};

use super::protocols::webhook_sync_data;
pub fn create_sync_router() -> Router<Database> {
    Router::new()
        .route("/status", get(get_sync_status))
        .route("/webhook", post(webhook_sync_data))
        .route("/realtime-status", get(get_realtime_sync_status))
        .route("/event-stats", get(get_event_processing_stats))
        .route("/trigger-full-sync", post(trigger_manual_sync))
        .route("/scheduler-status", get(get_scheduler_status))
}

async fn get_sync_status(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching sync status");

    let status = db.get_sync_status().await?;

    Ok(Json(json!({
        "success": true,
        "data": status
    })))
}

async fn get_realtime_sync_status(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching real-time sync status");

    let realtime_sync = crate::services::realtime_sync::RealtimeSyncService::new(db.pool().clone());
    let stats = realtime_sync.get_sync_stats().await
        .map_err(|e| AppError::Internal(format!("Failed to get real-time sync stats: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "message": "Real-time synchronization status",
        "data": {
            "isActive": stats.is_active,
            "lastBetSync": stats.last_bet_sync,
            "lastMarketSync": stats.last_market_sync,
            "pendingBets": stats.pending_bets,
            "config": {
                "betSyncIntervalMs": stats.config.bet_sync_interval_ms,
                "marketSyncIntervalMs": stats.config.market_sync_interval_ms,
                "enableImmediateSync": stats.config.enable_immediate_sync,
                "maxRetries": stats.config.max_retries
            },
            "healthCheck": {
                "syncLatency": "real-time",
                "status": if stats.pending_bets < 10 { "healthy" } else { "warning" }
            }
        }
    })))
}

async fn get_event_processing_stats(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching event processing statistics");

    let db_listener = crate::services::db_event_listener::DbEventListener::new(db.pool().clone());
    let stats = db_listener.get_event_stats().await
        .map_err(|e| AppError::Internal(format!("Failed to get event stats: {}", e)))?;

    
    let event_types: Vec<_> = stats.iter().map(|s| {
        json!({
            "eventType": s.event_type,
            "totalProcessed": s.total_processed,
            "successful": s.successful,
            "errors": s.errors,
            "avgDurationMs": s.avg_duration_ms.as_ref().map(|d| d.to_string()),
            "lastProcessedAt": s.last_processed_at
        })
    }).collect();

    Ok(Json(json!({
        "success": true,
        "message": "Event processing statistics (last hour)",
        "data": {
            "eventTypes": event_types,
            "summary": {
                "totalEvents": stats.iter().map(|s| s.total_processed.unwrap_or(0)).sum::<i64>(),
                "totalSuccessful": stats.iter().map(|s| s.successful.unwrap_or(0)).sum::<i64>(),
                "totalErrors": stats.iter().map(|s| s.errors.unwrap_or(0)).sum::<i64>(),
                "totalEventTypes": stats.len()
            }
        }
    })))
}

/// Manually trigger a full sync from the indexer database
async fn trigger_manual_sync(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Manual sync triggered via API");
    
    let scheduler = Scheduler::new(db.pool().clone());
    match scheduler.trigger_sync_now().await {
        Ok(summary) => {
            info!(
                "Manual sync completed: {} processed, {} errors in {}ms",
                summary.total_processed, summary.total_errors, summary.duration_ms
            );
            
            let results: Vec<_> = summary.results.iter().map(|r| {
                json!({
                    "eventType": r.event_type,
                    "processed": r.processed,
                    "newEvents": r.new_events,
                    "errors": r.errors,
                    "skipped": r.skipped
                })
            }).collect();
            
            Ok(Json(json!({
                "success": true,
                "message": "Manual sync completed successfully",
                "data": {
                    "totalProcessed": summary.total_processed,
                    "totalErrors": summary.total_errors,
                    "durationMs": summary.duration_ms,
                    "results": results
                }
            })))
        }
        Err(e) => {
            let error_msg = format!("Manual sync failed: {}", e);
            Err(AppError::Internal(error_msg))
        }
    }
}

/// Get scheduler configuration and status
async fn get_scheduler_status(State(db): State<Database>) -> Result<Json<Value>, AppError> {
    info!("Fetching scheduler status");
    
    let scheduler = Scheduler::new(db.pool().clone());
    let status = scheduler.get_status();
    
    Ok(Json(json!({
        "success": true,
        "message": "Scheduler status",
        "data": {
            "indexerSync": {
                "enabled": status.indexer_sync_enabled,
                "intervalSeconds": status.indexer_sync_interval_secs,
                "nextRunEstimate": "background job running"
            },
            "yieldCalculation": {
                "enabled": status.yield_calc_enabled,
                "intervalSeconds": status.yield_calc_interval_secs,
                "nextRunEstimate": "background job running"
            }
        }
    })))
}

