use bigdecimal::BigDecimal;
use bigdecimal::ToPrimitive;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::debug;

use crate::error::AppError;
use crate::models::{ChartDataPoint, MarketChartData};

pub struct ChartService {
    pool: PgPool,
}

impl ChartService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_market_chart_data(
        &self,
        market_id: i64,
        interval: &str,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<MarketChartData, AppError> {
        let interval_seconds = Self::validate_interval(interval)?;

        let market = sqlx::query!(
            r#"SELECT EXTRACT(EPOCH FROM "createdAt")::BIGINT as created_at FROM markets_extended WHERE "blockchainMarketId" = $1 LIMIT 1"#,
            market_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(anyhow::anyhow!(e)))?;

        let market_created_at = market
            .and_then(|m| m.created_at)
            .unwrap_or_else(|| chrono::Utc::now().timestamp() - 86400 * 7);

        let to_timestamp = to.unwrap_or_else(|| chrono::Utc::now().timestamp());

        let from_timestamp = from.unwrap_or(market_created_at);

        let bets = sqlx::query!(
            r#"
            SELECT
                bet_id,
                position,
                amount,
                EXTRACT(EPOCH FROM inserted_at)::BIGINT as timestamp
            FROM bets
            WHERE market_id = $1
                AND EXTRACT(EPOCH FROM inserted_at) BETWEEN $2 AND $3
            ORDER BY inserted_at ASC
            "#,
            market_id,
            BigDecimal::from(from_timestamp),
            BigDecimal::from(to_timestamp)
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::Database(anyhow::anyhow!(e)))?;

        debug!("Fetched {} bets for chart data", bets.len());

        let start_bucket = (from_timestamp / interval_seconds) * interval_seconds;
        let end_bucket = (to_timestamp / interval_seconds) * interval_seconds;

        let mut times: Vec<i64> = Vec::new();
        let mut current = start_bucket;
        while current <= end_bucket {
            times.push(current);
            current += interval_seconds;
        }

        let mut time_buckets: HashMap<i64, BucketData> = HashMap::new();
        let mut yes_total: i64 = 0;
        let mut no_total: i64 = 0;

        for bet in bets {
            let bucket_time = (bet.timestamp.unwrap_or(0) / interval_seconds) * interval_seconds;
            let entry = time_buckets
                .entry(bucket_time)
                .or_insert_with(|| BucketData {
                    yes_volume: 0,
                    no_volume: 0,
                    yes_count: 0,
                    no_count: 0,
                    yes_total: 0,
                    no_total: 0,
                });

            let amount_i64 = bet.amount.to_i64().unwrap_or(0);

            let is_yes_position = bet.position;

            if is_yes_position {
                entry.yes_volume += amount_i64;
                entry.yes_count += 1;
                yes_total += amount_i64;
            } else {
                entry.no_volume += amount_i64;
                entry.no_count += 1;
                no_total += amount_i64;
            }

            entry.yes_total = yes_total;
            entry.no_total = no_total;
        }

        let mut yes_probability = Vec::new();
        let mut no_probability = Vec::new();
        let mut yes_volume = Vec::new();
        let mut no_volume = Vec::new();
        let mut total_volume = Vec::new();
        let mut yes_odds = Vec::new();
        let mut no_odds = Vec::new();
        let mut bet_count = Vec::new();

        let mut running_yes_total: i64 = 0;
        let mut running_no_total: i64 = 0;
        let mut cumulative_yes_volume: i64 = 0;
        let mut cumulative_no_volume: i64 = 0;
        let mut cumulative_bet_count: i32 = 0;

        for time in times {
            let bucket = time_buckets.get(&time);

            if let Some(b) = bucket {
                running_yes_total = b.yes_total;
                running_no_total = b.no_total;
                cumulative_yes_volume += b.yes_volume;
                cumulative_no_volume += b.no_volume;
                cumulative_bet_count += b.yes_count + b.no_count;
            }

            let total = running_yes_total + running_no_total;

            let yes_prob = if total > 0 {
                running_yes_total as f64 / total as f64
            } else {
                0.5
            };
            let no_prob = 1.0 - yes_prob;

            yes_probability.push(ChartDataPoint {
                time,
                value: yes_prob,
            });
            no_probability.push(ChartDataPoint {
                time,
                value: no_prob,
            });

            yes_volume.push(ChartDataPoint {
                time,
                value: cumulative_yes_volume as f64,
            });
            no_volume.push(ChartDataPoint {
                time,
                value: cumulative_no_volume as f64,
            });
            total_volume.push(ChartDataPoint {
                time,
                value: (cumulative_yes_volume + cumulative_no_volume) as f64,
            });

            let yes_odd = if yes_prob > 0.0 { 1.0 / yes_prob } else { 2.0 };
            let no_odd = if no_prob > 0.0 { 1.0 / no_prob } else { 2.0 };

            yes_odds.push(ChartDataPoint {
                time,
                value: yes_odd,
            });
            no_odds.push(ChartDataPoint {
                time,
                value: no_odd,
            });

            bet_count.push(ChartDataPoint {
                time,
                value: cumulative_bet_count as f64,
            });
        }

        Ok(MarketChartData {
            yes_probability,
            no_probability,
            yes_volume,
            no_volume,
            total_volume,
            yes_odds,
            no_odds,
            bet_count,
        })
    }

    fn validate_interval(interval: &str) -> Result<i64, AppError> {
        let seconds = match interval {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "1h" => 3600,
            "4h" => 14400,
            "1d" => 86400,
            _ => {
                return Err(AppError::BadRequest(format!(
                    "Invalid interval '{}'. Allowed values: 1m, 5m, 15m, 1h, 4h, 1d",
                    interval
                )))
            }
        };
        Ok(seconds)
    }
}

struct BucketData {
    yes_volume: i64,
    no_volume: i64,
    yes_count: i32,
    no_count: i32,
    yes_total: i64,
    no_total: i64,
}
