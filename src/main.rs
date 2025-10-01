use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod chart;
mod config;
mod db;
mod error;
mod middleware;
mod models;
mod openapi;
mod routes;
mod seed;
mod services;
mod utils;

pub mod admin {
    pub mod routes;
}

use config::Config;
use db::Database;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kizo_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Kizo Prediction Market API Server (Rust)");

    
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");
    info!("Server will listen on {}:{}", config.host, config.port);

    
    info!("Connecting to database...");
    let db = Database::new(&config.database_url).await?;
    info!("Database connection established");

    
    if db.health_check().await.is_ok() {
        info!("Database health check passed");
    } else {
        error!("Database health check failed");
        return Err(anyhow::anyhow!("Database health check failed"));
    }

    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    
    if std::env::var("RUN_SEEDS").unwrap_or_default() == "true" {
        info!("Running database seeds...");
        seed::run_all_seeds(db.pool()).await?;
        
        
        let yield_service = services::yield_service::YieldService::new(db.pool().clone());
        yield_service.initialize_protocols().await?;
        info!("Seeds and protocols initialized");
    }
    
    
    info!("🔄 Running initial full synchronization from indexer database...");
    let initial_sync = services::blockchain_sync::BlockchainSyncService::new(db.pool().clone());
    match initial_sync.run_full_sync().await {
        Ok(summary) => {
            info!(
                "✅ Initial sync completed: {} processed, {} errors in {}ms",
                summary.total_processed, summary.total_errors, summary.duration_ms
            );
            if summary.total_processed > 0 {
                info!("📊 Synced {} total events from indexer", summary.total_processed);
                for result in &summary.results {
                    if result.new_events > 0 {
                        info!("  - {}: {} new items", result.event_type, result.new_events);
                    }
                }
            } else {
                info!("ℹ️  No new data to sync - database is up to date");
            }
        }
        Err(e) => {
            error!("⚠️  Initial sync encountered errors: {}", e);
        }
    }

    
    info!("🚀 Starting background scheduler and event listener...");
    let scheduler = Arc::new(services::scheduler::Scheduler::new(db.pool().clone()));
    scheduler.start().await;

    
    let app = Router::new()
        .merge(
            SwaggerUi::new("/api-docs")
                .url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .nest("/api", routes::create_router(db.clone()))
        .merge(admin::routes::create_admin_router(db))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors),
        )
        .into_make_service();

    
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Server listening on {}", addr);

    
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("🚀 Kizo Prediction Market API Server started successfully");
    info!("📚 API Documentation: http://{}:{}/api-docs", config.host, config.port);
    info!("📖 OpenAPI Spec: http://{}:{}/api-docs/openapi.json", config.host, config.port);
    info!("💚 Health Check: http://{}:{}/api/health", config.host, config.port);
    info!("");
    info!("Available endpoints:");
    info!("  - GET  /api/markets              - List all markets");
    info!("  - GET  /api/markets/:id          - Get market by ID");
    info!("  - GET  /api/markets/:id/stats    - Get market statistics");
    info!("  - GET  /api/markets/stats/platform - Get platform stats");
    info!("  - GET  /api/bets                 - List recent bets");
    info!("  - GET  /api/bets/:id             - Get bet by ID");
    info!("  - GET  /api/bets/user/:address   - Get user bets");
    info!("  - GET  /api/bets/user/:address/stats - Get user stats");
    info!("  - GET  /api/bets/market/:id      - Get market bets");
    info!("  - GET  /api/sync/status          - Get sync status");

    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal, initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("Received SIGTERM signal, initiating graceful shutdown...");
        },
    }
}