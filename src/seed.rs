use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, error};
use uuid::Uuid;

use crate::services::{
    YieldService,
    market_seeder::MarketSeeder,
};


pub async fn seed_protocols(pool: &PgPool) -> Result<()> {
    info!("Seeding protocols...");

    let protocols = vec![
        (
            "amnis",
            "Amnis Finance",
            "5.5",
            "Liquid staking protocol for Aptos",
            Some("https://res.cloudinary.com/dutlw7bko/image/upload/v1759242167/protocols/amnis_hr2eu4.png"),
        ),
        (
            "kiln",
            "Kiln Protocol",
            "4.8",
            "Staking infrastructure for Aptos",
            Some("https://res.cloudinary.com/dutlw7bko/image/upload/v1759242167/protocols/kiln_pdnq9s.png"),
        ),
        (
            "kofi",
            "Kofi Finance",
            "6.2",
            "DeFi yield aggregator on Aptos",
            Some("https://res.cloudinary.com/dutlw7bko/image/upload/v1759242171/protocols/kofi_wdvugx.png"),
        ),
    ];

    for (name, display_name, base_apy, description, icon_url) in protocols {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query!(
            r#"
            INSERT INTO protocols (id, name, "displayName", "baseApy", "isActive", description, "iconUrl", "createdAt", "updatedAt")
            VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT (name) DO UPDATE SET
                "displayName" = EXCLUDED."displayName",
                "baseApy" = EXCLUDED."baseApy",
                description = EXCLUDED.description,
                "iconUrl" = EXCLUDED."iconUrl",
                "updatedAt" = CURRENT_TIMESTAMP
            "#,
            id,
            name,
            display_name,
            base_apy.parse::<sqlx::types::BigDecimal>().unwrap(),
            true,
            description,
            icon_url
        )
        .execute(pool)
        .await?;

        info!("Seeded protocol: {}", name);
    }

    info!("Protocol seeding complete");
    
    
    info!("Updating protocol APY from blockchain...");
    let yield_service = YieldService::new(pool.clone());
    match yield_service.update_all_protocols_apy().await {
        Ok(results) => {
            for (protocol, apy) in results {
                info!("✅ Updated {} APY from blockchain: {}%", protocol, apy);
            }
        }
        Err(e) => {
            error!("⚠️  Failed to update APY from blockchain: {}. Using default values.", e);
        }
    }
    
    Ok(())
}


pub async fn seed_markets(pool: &PgPool, count: usize) -> Result<()> {
    info!("Seeding {} markets from Adjacent API...", count);
    
    let api_key = std::env::var("ADJACENT_API_KEY")
        .expect("ADJACENT_API_KEY must be set");
    
    let seeder = MarketSeeder::new(pool.clone(), api_key)?;
    let result = seeder.seed_markets(count).await?;
    
    info!(
        "Market seeding complete: {} created, {} updated, {} skipped, {} errors",
        result.created, result.updated, result.skipped, result.errors
    );
    
    Ok(())
}


pub async fn run_all_seeds(pool: &PgPool) -> Result<()> {
    info!("🌱 Running all seeds...");
    
    
    seed_protocols(pool).await?;
    
    
    let market_count = std::env::var("SEED_MARKET_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10); 
    
    match seed_markets(pool, market_count).await {
        Ok(_) => info!("✅ Market seeding successful"),
        Err(e) => error!("⚠️  Market seeding failed: {}. Continuing...", e),
    }
    
    info!("✅ All seeds complete");
    Ok(())
}
