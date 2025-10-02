use anyhow::Result;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kizo_server::{config::Config, db::Database, seed};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kizo_server=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🌱 Kizo Market Seeder - Starting...");

    // Load config
    let config = Config::from_env()?;
    info!("✅ Configuration loaded");

    // Connect to database
    info!("🔌 Connecting to database...");
    let db = Database::new(&config.database_url).await?;
    info!("✅ Database connection established");

    // Health check
    if db.health_check().await.is_err() {
        error!("❌ Database health check failed");
        return Err(anyhow::anyhow!("Database health check failed"));
    }
    info!("✅ Database health check passed");

    // Run seeds
    info!("🌱 Starting market seeding process...");
    match seed::run_all_seeds(db.pool()).await {
        Ok(_) => {
            info!("✅ Seeding completed successfully!");
        }
        Err(e) => {
            error!("❌ Seeding failed: {}", e);
            return Err(e);
        }
    }

    info!("🎉 Market seeding process complete!");
    Ok(())
}
