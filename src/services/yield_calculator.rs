use anyhow::Result;
use chrono::NaiveDateTime;
use sqlx::types::BigDecimal;
use sqlx::PgPool;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct YieldData {
    pub daily_yield: f64,
    pub total_yield_until_end: f64,
    pub days_remaining: i64,
    pub best_protocol_apy: f64,
    pub best_protocol_name: String,
}

pub async fn calculate_market_yield_data(
    pool: &PgPool,
    total_pool_size: &BigDecimal,
    volume: &BigDecimal,
    end_date: &NaiveDateTime,
) -> Result<YieldData> {
    let now = chrono::Utc::now().naive_utc();
    let days_remaining = (*end_date - now).num_days().max(0);

    let best_protocol = sqlx::query!(
        r#"
        SELECT name, "baseApy" as base_apy
        FROM protocols
        WHERE "isActive" = true
        ORDER BY "baseApy" DESC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    let (best_apy, best_name) = if let Some(protocol) = best_protocol {
        let apy_str = protocol.base_apy.to_string();
        let apy = f64::from_str(&apy_str).unwrap_or(5.0);
        (apy, protocol.name)
    } else {
        (5.0, "default".to_string())
    };

    let pool_str = total_pool_size.to_string();
    let mut pool_amount = f64::from_str(&pool_str).unwrap_or(0.0);

    if pool_amount < 0.001 {
        let volume_str = volume.to_string();
        let volume_val = f64::from_str(&volume_str).unwrap_or(0.0);
        pool_amount = volume_val / 1_000_000.0;
    }

    pool_amount = pool_amount.max(0.0);

    let (daily_yield, total_yield_until_end) = if pool_amount <= 0.0 || days_remaining <= 0 {
        (0.0, 0.0)
    } else {
        let daily_rate = best_apy / 365.0 / 100.0;
        let daily = pool_amount * daily_rate;
        let total = daily * (days_remaining as f64);
        (daily, total)
    };

    Ok(YieldData {
        daily_yield,
        total_yield_until_end,
        days_remaining,
        best_protocol_apy: best_apy,
        best_protocol_name: best_name,
    })
}
