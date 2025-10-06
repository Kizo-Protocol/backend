use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use super::blockchain_sync::BlockchainSyncService;
use super::db_event_listener::DbEventListener;
use super::yield_service::YieldService;

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub indexer_sync_interval_secs: u64,

    pub yield_calc_interval_secs: u64,

    pub enable_indexer_sync: bool,

    pub enable_yield_calc: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            indexer_sync_interval_secs: std::env::var("INDEXER_SYNC_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            yield_calc_interval_secs: std::env::var("YIELD_CALC_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
            enable_indexer_sync: std::env::var("ENABLE_INDEXER_SYNC")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_yield_calc: std::env::var("ENABLE_YIELD_CALC")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        }
    }
}

pub struct Scheduler {
    pool: PgPool,
    config: SchedulerConfig,
}

impl Scheduler {
    pub fn new(pool: PgPool) -> Self {
        let config = SchedulerConfig::default();
        info!("Scheduler configuration: {:?}", config);
        Self { pool, config }
    }

    #[allow(dead_code)]
    pub fn new_with_config(pool: PgPool, config: SchedulerConfig) -> Self {
        info!("Scheduler configuration: {:?}", config);
        Self { pool, config }
    }

    pub async fn start(self: Arc<Self>) {
        info!("🚀 Starting scheduler with background jobs");
        info!(
            "   - Indexer sync: {} (interval: {}s)",
            if self.config.enable_indexer_sync {
                "enabled"
            } else {
                "disabled"
            },
            self.config.indexer_sync_interval_secs
        );
        info!(
            "   - Yield calculation: {} (interval: {}s)",
            if self.config.enable_yield_calc {
                "enabled"
            } else {
                "disabled"
            },
            self.config.yield_calc_interval_secs
        );

        let sync_scheduler = Arc::clone(&self);
        let yield_scheduler = Arc::clone(&self);
        let db_event_scheduler = Arc::clone(&self);

        tokio::spawn(async move {
            let db_listener = DbEventListener::new(db_event_scheduler.pool.clone());
            info!("🎧 Starting database event listener for real-time event processing");
            if let Err(e) = db_listener.start_listening().await {
                error!("❌ Database event listener error: {}", e);
                warn!("⚠️  Event listener failed, relying on periodic sync instead");
            }
        });

        if self.config.enable_indexer_sync {
            let interval_secs = self.config.indexer_sync_interval_secs;
            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(interval_secs));
                let mut sync_count = 0u64;

                loop {
                    interval.tick().await;
                    sync_count += 1;
                    info!(
                        "🔄 [Sync Job #{}] Running indexer database sync",
                        sync_count
                    );

                    let sync_service = BlockchainSyncService::new(sync_scheduler.pool.clone());
                    match sync_service.run_full_sync().await {
                        Ok(summary) => {
                            if summary.total_processed > 0 {
                                info!(
                                    "✅ [Sync Job #{}] Completed: {} new events, {} errors, {}ms",
                                    sync_count,
                                    summary.total_processed,
                                    summary.total_errors,
                                    summary.duration_ms
                                );

                                for result in &summary.results {
                                    if result.new_events > 0 {
                                        info!(
                                            "   └─ {}: {} new items",
                                            result.event_type, result.new_events
                                        );
                                    }
                                }
                            } else {
                                info!("✅ [Sync Job #{}] No new data to sync", sync_count);
                            }
                        }
                        Err(e) => {
                            error!("❌ [Sync Job #{}] Failed: {}", sync_count, e);
                        }
                    }
                }
            });
            info!("✅ Indexer sync job started (every {}s)", interval_secs);
        } else {
            warn!("⚠️  Indexer sync job is disabled");
        }

        if self.config.enable_yield_calc {
            let interval_secs = self.config.yield_calc_interval_secs;
            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(interval_secs));
                let mut calc_count = 0u64;

                loop {
                    interval.tick().await;
                    calc_count += 1;
                    info!("📊 [Yield Job #{}] Running yield calculation", calc_count);

                    let yield_service = YieldService::new(yield_scheduler.pool.clone());
                    match yield_service.calculate_all_market_yields().await {
                        Ok(count) => {
                            info!(
                                "✅ [Yield Job #{}] Calculated yields for {} markets",
                                calc_count, count
                            );
                        }
                        Err(e) => {
                            error!("❌ [Yield Job #{}] Failed: {}", calc_count, e);
                        }
                    }
                }
            });
            info!(
                "✅ Yield calculation job started (every {}s)",
                interval_secs
            );
        } else {
            warn!("⚠️  Yield calculation job is disabled");
        }

        info!("✨ Scheduler started successfully - all background jobs running");
    }

    pub async fn trigger_sync_now(&self) -> anyhow::Result<super::blockchain_sync::SyncSummary> {
        info!("🔄 Manual sync triggered");
        let sync_service = BlockchainSyncService::new(self.pool.clone());
        sync_service.run_full_sync().await
    }

    pub fn get_status(&self) -> SchedulerStatus {
        SchedulerStatus {
            indexer_sync_enabled: self.config.enable_indexer_sync,
            indexer_sync_interval_secs: self.config.indexer_sync_interval_secs,
            yield_calc_enabled: self.config.enable_yield_calc,
            yield_calc_interval_secs: self.config.yield_calc_interval_secs,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SchedulerStatus {
    pub indexer_sync_enabled: bool,
    pub indexer_sync_interval_secs: u64,
    pub yield_calc_enabled: bool,
    pub yield_calc_interval_secs: u64,
}
