use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{debug, info};

// Constants
const OCTAS_PER_APT: f64 = 100_000_000.0; // 10^8 octas = 1 APT

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserYieldSummary {
    pub total_yield_earned: f64,
    pub total_amount_staked: f64,
    pub average_apy: f64,
    pub active_pool_size: f64,
    pub protocol_breakdown: Vec<ProtocolYieldBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolYieldBreakdown {
    pub protocol: String,
    pub total_amount: f64,
    pub total_yield: f64,
    pub average_apy: f64,
}

#[derive(Debug)]
struct ActiveBet {
    amount: i64,
    created_at: NaiveDateTime,
}

#[derive(Debug)]
struct Protocol {
    name: String,
    base_apy: String,
}

pub struct UserYieldCalculator {
    pool: PgPool,
}

impl UserYieldCalculator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Calculate yield summary for a specific user
    pub async fn calculate_user_yields(&self, user_address: &str) -> Result<UserYieldSummary> {
        info!("Calculating yields for user: {}", user_address);

        // Fetch active protocols
        let protocols = self.fetch_active_protocols().await?;

        if protocols.is_empty() {
            debug!("No active protocols found, returning zero yields");
            return Ok(UserYieldSummary {
                total_yield_earned: 0.0,
                total_amount_staked: 0.0,
                average_apy: 0.0,
                active_pool_size: 0.0,
                protocol_breakdown: Vec::new(),
            });
        }

        // Get the best (highest) APY protocol for calculations
        let best_protocol = protocols
            .iter()
            .max_by(|a, b| {
                let apy_a = f64::from_str(&a.base_apy).unwrap_or(0.0);
                let apy_b = f64::from_str(&b.base_apy).unwrap_or(0.0);
                apy_a.partial_cmp(&apy_b).unwrap()
            })
            .unwrap();

        let best_apy = f64::from_str(&best_protocol.base_apy).unwrap_or(0.0);

        debug!(
            "Using protocol '{}' with APY: {}",
            best_protocol.name, best_apy
        );

        // Fetch active bets for the user
        let active_bets = self.fetch_user_active_bets(user_address).await?;

        if active_bets.is_empty() {
            debug!("No active bets found for user");
            return Ok(self.create_empty_summary_with_protocols(&protocols));
        }

        // Calculate yields
        let now = chrono::Utc::now().naive_utc();
        let mut total_yield = 0.0;
        let mut total_amount = 0.0;

        for bet in &active_bets {
            // Convert from octas (10^8 smallest units) to APT
            let amount = bet.amount as f64 / OCTAS_PER_APT;
            total_amount += amount;

            // Calculate time elapsed in days
            let elapsed_days = (now - bet.created_at).num_days() as f64;
            let elapsed_hours = (now - bet.created_at).num_hours() as f64;

            // Use hours for more granular calculation if less than a day
            let time_factor = if elapsed_days < 1.0 {
                elapsed_hours / 24.0
            } else {
                elapsed_days
            };

            // Calculate yield: amount * (apy / 100) * (days_elapsed / 365)
            let yield_amount = amount * (best_apy / 100.0) * (time_factor / 365.0);
            total_yield += yield_amount;

            debug!(
                "Bet amount: {}, elapsed days: {:.2}, yield: {:.4}",
                amount, time_factor, yield_amount
            );
        }

        // Create protocol breakdown
        let mut protocol_breakdown = Vec::new();
        for protocol in &protocols {
            let protocol_apy = f64::from_str(&protocol.base_apy).unwrap_or(0.0);

            // For simplicity, assign all user's bets to the best protocol
            // In a real scenario, you'd track which protocol each bet uses
            if protocol.name == best_protocol.name {
                protocol_breakdown.push(ProtocolYieldBreakdown {
                    protocol: protocol.name.clone(),
                    total_amount,
                    total_yield,
                    average_apy: protocol_apy,
                });
            } else {
                protocol_breakdown.push(ProtocolYieldBreakdown {
                    protocol: protocol.name.clone(),
                    total_amount: 0.0,
                    total_yield: 0.0,
                    average_apy: protocol_apy,
                });
            }
        }

        info!(
            "Calculated yields - Total: {:.4}, Amount: {:.4}, APY: {:.2}",
            total_yield, total_amount, best_apy
        );

        Ok(UserYieldSummary {
            total_yield_earned: total_yield,
            total_amount_staked: total_amount,
            average_apy: best_apy,
            active_pool_size: total_amount,
            protocol_breakdown,
        })
    }

    /// Calculate global yield summary (all users)
    pub async fn calculate_global_yields(&self) -> Result<UserYieldSummary> {
        info!("Calculating global yields for all users");

        let protocols = self.fetch_active_protocols().await?;

        if protocols.is_empty() {
            return Ok(UserYieldSummary {
                total_yield_earned: 0.0,
                total_amount_staked: 0.0,
                average_apy: 0.0,
                active_pool_size: 0.0,
                protocol_breakdown: Vec::new(),
            });
        }

        let best_protocol = protocols
            .iter()
            .max_by(|a, b| {
                let apy_a = f64::from_str(&a.base_apy).unwrap_or(0.0);
                let apy_b = f64::from_str(&b.base_apy).unwrap_or(0.0);
                apy_a.partial_cmp(&apy_b).unwrap()
            })
            .unwrap();

        let best_apy = f64::from_str(&best_protocol.base_apy).unwrap_or(0.0);

        // Fetch all active bets
        let active_bets = self.fetch_all_active_bets().await?;

        if active_bets.is_empty() {
            return Ok(self.create_empty_summary_with_protocols(&protocols));
        }

        // Calculate yields
        let now = chrono::Utc::now().naive_utc();
        let mut total_yield = 0.0;
        let mut total_amount = 0.0;

        for bet in &active_bets {
            // Convert from octas (10^8 smallest units) to APT
            let amount = bet.amount as f64 / OCTAS_PER_APT;
            total_amount += amount;

            let elapsed_days = (now - bet.created_at).num_days() as f64;
            let elapsed_hours = (now - bet.created_at).num_hours() as f64;

            let time_factor = if elapsed_days < 1.0 {
                elapsed_hours / 24.0
            } else {
                elapsed_days
            };

            let yield_amount = amount * (best_apy / 100.0) * (time_factor / 365.0);
            total_yield += yield_amount;
        }

        // Create protocol breakdown
        let mut protocol_breakdown = Vec::new();
        for protocol in &protocols {
            let protocol_apy = f64::from_str(&protocol.base_apy).unwrap_or(0.0);

            if protocol.name == best_protocol.name {
                protocol_breakdown.push(ProtocolYieldBreakdown {
                    protocol: protocol.name.clone(),
                    total_amount,
                    total_yield,
                    average_apy: protocol_apy,
                });
            } else {
                protocol_breakdown.push(ProtocolYieldBreakdown {
                    protocol: protocol.name.clone(),
                    total_amount: 0.0,
                    total_yield: 0.0,
                    average_apy: protocol_apy,
                });
            }
        }

        Ok(UserYieldSummary {
            total_yield_earned: total_yield,
            total_amount_staked: total_amount,
            average_apy: best_apy,
            active_pool_size: total_amount,
            protocol_breakdown,
        })
    }

    async fn fetch_active_protocols(&self) -> Result<Vec<Protocol>> {
        let rows = sqlx::query!(
            r#"
            SELECT name, "baseApy" as base_apy
            FROM protocols
            WHERE "isActive" = true
            ORDER BY "baseApy" DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let protocols = rows
            .into_iter()
            .map(|row| Protocol {
                name: row.name,
                base_apy: row.base_apy.to_string(),
            })
            .collect();

        Ok(protocols)
    }

    async fn fetch_user_active_bets(&self, user_address: &str) -> Result<Vec<ActiveBet>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                b.amount,
                b.inserted_at
            FROM bets b
            INNER JOIN markets m ON b.market_id = m.market_id
            WHERE b.user_addr = $1
              AND b.claimed IS NULL OR b.claimed = false
            ORDER BY b.inserted_at ASC
            "#,
            user_address
        )
        .fetch_all(&self.pool)
        .await?;

        let bets = rows
            .into_iter()
            .map(|row| ActiveBet {
                amount: row.amount,
                created_at: row.inserted_at,
            })
            .collect();

        Ok(bets)
    }

    async fn fetch_all_active_bets(&self) -> Result<Vec<ActiveBet>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                b.amount,
                b.inserted_at
            FROM bets b
            INNER JOIN markets m ON b.market_id = m.market_id
            WHERE b.claimed IS NULL OR b.claimed = false
            ORDER BY b.inserted_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let bets = rows
            .into_iter()
            .map(|row| ActiveBet {
                amount: row.amount,
                created_at: row.inserted_at,
            })
            .collect();

        Ok(bets)
    }

    fn create_empty_summary_with_protocols(&self, protocols: &[Protocol]) -> UserYieldSummary {
        let protocol_breakdown = protocols
            .iter()
            .map(|p| {
                let apy = f64::from_str(&p.base_apy).unwrap_or(0.0);
                ProtocolYieldBreakdown {
                    protocol: p.name.clone(),
                    total_amount: 0.0,
                    total_yield: 0.0,
                    average_apy: apy,
                }
            })
            .collect();

        UserYieldSummary {
            total_yield_earned: 0.0,
            total_amount_staked: 0.0,
            average_apy: 0.0,
            active_pool_size: 0.0,
            protocol_breakdown,
        }
    }
}
