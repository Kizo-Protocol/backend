use anyhow::Result;
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use tracing::{error, info};
use uuid::Uuid;

use crate::models::Protocol;

pub struct YieldService {
    pool: PgPool,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct YieldCalculation {
    pub current_yield: BigDecimal,
    pub protocol_breakdown: Vec<ProtocolYield>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProtocolYield {
    pub protocol: String,
    pub amount: BigDecimal,
    pub apy: BigDecimal,
    pub yield_amount: BigDecimal,
}

#[allow(dead_code)]
impl YieldService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    
    pub async fn initialize_protocols(&self) -> Result<()> {
        info!("Initializing yield protocols");

        let module_address = std::env::var("APTOS_MODULE_ADDRESS")
            .unwrap_or_else(|_| "0xaab1ca043fb6cd7b2f264f3ce32f301427e7f67b0e816b30d4e95f1e2bcbabfa".to_string());
        
        
        let amnis_addr = std::env::var("AMNIS_PROTOCOL_ADDRESS")
            .unwrap_or_else(|_| module_address.clone());
        let kiln_addr = std::env::var("KILN_PROTOCOL_ADDRESS")
            .unwrap_or_else(|_| module_address.clone());
        let kofi_addr = std::env::var("KOFI_PROTOCOL_ADDRESS")
            .unwrap_or_else(|_| module_address.clone());

        
        let protocols = vec![
            ("amnis", "Amnis Finance", "Liquid staking protocol for Aptos", amnis_addr, "amnis_adapter"),
            ("kiln", "Kiln Protocol", "Staking infrastructure for Aptos", kiln_addr, "kiln_adapter"),
            ("kofi", "Kofi Finance", "DeFi yield aggregator on Aptos", kofi_addr, "kofi_adapter"),
        ];

        for (name, display_name, description, protocol_addr, adapter_name) in protocols {
            
            let apy = match self.fetch_protocol_apy(&module_address, adapter_name, &protocol_addr).await {
                Ok(contract_apy) => {
                    
                    let apy_percent = contract_apy as f64 / 100.0;
                    info!("Fetched APY from {} contract: {}%", name, apy_percent);
                    BigDecimal::try_from(apy_percent)?
                },
                Err(e) => {
                    error!("Failed to fetch APY for {}: {}. Using database value.", name, e);
                    
                    match sqlx::query_scalar::<_, BigDecimal>(
                        r#"SELECT "baseApy" FROM protocols WHERE name = $1"#
                    )
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await
                    {
                        Ok(Some(existing_apy)) => existing_apy,
                        _ => {
                            
                            let default = match name {
                                "amnis" => "5.5",
                                "kiln" => "4.8",
                                "kofi" => "6.2",
                                _ => "5.0",
                            };
                            BigDecimal::try_from(default.parse::<f64>()?).unwrap()
                        }
                    }
                }
            };

            let id = Uuid::new_v4().to_string();

            sqlx::query!(
                r#"
                INSERT INTO protocols (id, name, "displayName", "baseApy", "isActive", description, "createdAt", "updatedAt")
                VALUES ($1, $2, $3, $4, true, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                ON CONFLICT (name) DO UPDATE SET
                    "displayName" = EXCLUDED."displayName",
                    "baseApy" = EXCLUDED."baseApy",
                    description = EXCLUDED.description,
                    "updatedAt" = CURRENT_TIMESTAMP
                "#,
                id,
                name,
                display_name,
                apy,
                description
            )
            .execute(&self.pool)
            .await?;

            info!("Updated protocol {}: {}% APY", name, apy);
        }

        info!("Protocols initialized successfully");
        Ok(())
    }

    
    pub async fn update_protocol_apy_from_blockchain(&self, protocol_name: &str) -> Result<BigDecimal> {
        let module_address = std::env::var("APTOS_MODULE_ADDRESS")
            .unwrap_or_else(|_| "0xaab1ca043fb6cd7b2f264f3ce32f301427e7f67b0e816b30d4e95f1e2bcbabfa".to_string());
        
        let protocol_addr = std::env::var(format!("{}_PROTOCOL_ADDRESS", protocol_name.to_uppercase()))
            .unwrap_or_else(|_| module_address.clone());

        let adapter_name = format!("{}_adapter", protocol_name);

        let contract_apy = self.fetch_protocol_apy(&module_address, &adapter_name, &protocol_addr).await?;
        let apy_percent = contract_apy as f64 / 100.0;
        let apy = BigDecimal::try_from(apy_percent)?;

        
        sqlx::query!(
            r#"UPDATE protocols SET "baseApy" = $1, "updatedAt" = CURRENT_TIMESTAMP WHERE name = $2"#,
            apy,
            protocol_name
        )
        .execute(&self.pool)
        .await?;

        info!("Updated protocol {} APY from blockchain: {}%", protocol_name, apy_percent);
        Ok(apy)
    }

    
    pub async fn update_all_protocols_apy(&self) -> Result<Vec<(String, BigDecimal)>> {
        let protocols = vec!["amnis", "kiln", "kofi"];
        let mut results = Vec::new();
        let mut failed_protocols = Vec::new();

        for protocol in protocols {
            match self.update_protocol_apy_from_blockchain(protocol).await {
                Ok(apy) => {
                    results.push((protocol.to_string(), apy));
                }
                Err(e) => {
                    error!("Failed to update APY for {}: {}. Using default value.", protocol, e);
                    failed_protocols.push(protocol.to_string());
                    
                }
            }
        }

        if !failed_protocols.is_empty() {
            info!("⚠️  Could not update APY from blockchain for: {}. Using default values.", failed_protocols.join(", "));
        }
        
        if results.is_empty() {
            info!("⚠️  No protocols updated from blockchain. All using default values.");
        } else {
            info!("✅ Successfully updated {} protocols from blockchain", results.len());
        }
        
        Ok(results)
    }

    
    async fn fetch_protocol_apy(
        &self,
        module_address: &str,
        adapter_name: &str,
        protocol_address: &str,
    ) -> Result<u64> {
        let node_url = std::env::var("APTOS_NODE_URL")
            .unwrap_or_else(|_| "https://fullnode.testnet.aptoslabs.com/v1".to_string());

        let function_id = format!("{}::{}::get_current_apy", module_address, adapter_name);
        
        let client = reqwest::Client::new();
        let view_url = format!("{}/view", node_url);
        
        let response = client
            .post(&view_url)
            .json(&serde_json::json!({
                "function": function_id,
                "type_arguments": [],
                "arguments": [protocol_address]
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch APY: HTTP {}",
                response.status()
            ));
        }

        let result: Vec<serde_json::Value> = response.json().await?;
        
        if let Some(apy_value) = result.first() {
            if let Some(apy_str) = apy_value.as_str() {
                return Ok(apy_str.parse::<u64>()?);
            } else if let Some(apy_u64) = apy_value.as_u64() {
                return Ok(apy_u64);
            }
        }

        Err(anyhow::anyhow!("Invalid APY response format"))
    }

    
    pub async fn get_protocols(&self) -> Result<Vec<Protocol>> {
        let protocols = sqlx::query_as!(
            Protocol,
            r#"
            SELECT id, name, "displayName" as display_name, "baseApy" as base_apy, "isActive" as is_active, description, "iconUrl" as icon_url, "createdAt" as created_at, "updatedAt" as updated_at
            FROM protocols
            WHERE "isActive" = true
            ORDER BY "baseApy" DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(protocols)
    }

    
    pub async fn calculate_market_yield(
        &self,
        _market_id: i64,
        pool_size: BigDecimal,
    ) -> Result<YieldCalculation> {
        let protocols = self.get_protocols().await?;
        
        if protocols.is_empty() {
            return Ok(YieldCalculation {
                current_yield: BigDecimal::from(0),
                protocol_breakdown: Vec::new(),
            });
        }

        let protocol_count = BigDecimal::from(protocols.len() as i64);
        let amount_per_protocol = &pool_size / &protocol_count;

        let mut protocol_breakdown = Vec::new();
        let mut total_yield = BigDecimal::from(0);

        for protocol in protocols {
            
            let market_multiplier = BigDecimal::from(1);
            let effective_apy = &protocol.base_apy * &market_multiplier;

            
            let yield_amount = (&amount_per_protocol * &effective_apy) / BigDecimal::from(100) / BigDecimal::from(365);

            protocol_breakdown.push(ProtocolYield {
                protocol: protocol.name,
                amount: amount_per_protocol.clone(),
                apy: effective_apy,
                yield_amount: yield_amount.clone(),
            });

            total_yield += yield_amount;
        }

        Ok(YieldCalculation {
            current_yield: total_yield,
            protocol_breakdown,
        })
    }

    
    pub async fn record_yield(
        &self,
        market_id: &str,
        protocol_id: &str,
        amount: BigDecimal,
        apy: BigDecimal,
        yield_amount: BigDecimal,
        period: chrono::NaiveDateTime,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO yield_records (id, "marketId", "protocolId", amount, apy, yield, period, "createdAt")
            VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP)
            "#,
            id,
            market_id,
            protocol_id,
            amount,
            apy,
            yield_amount,
            period
        )
        .execute(&self.pool)
        .await?;

        info!("Recorded yield for market {} with protocol {}", market_id, protocol_id);
        Ok(())
    }

    
    pub async fn calculate_all_market_yields(&self) -> Result<i64> {
        info!("Calculating yields for all active markets");

        let markets = sqlx::query!(
            r#"
            SELECT me.id, me."blockchainMarketId" as blockchain_market_id, me."totalPoolSize" as total_pool_size
            FROM markets_extended me
            WHERE me.status = 'active'
              AND me."totalPoolSize" > 0
            LIMIT 100
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut processed = 0;

        for market in markets {
            match self.calculate_market_yield(market.blockchain_market_id.unwrap_or(0), market.total_pool_size.clone()).await {
                Ok(yield_calc) => {
                    
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE markets_extended
                        SET "currentYield" = $1, "updatedAt" = CURRENT_TIMESTAMP
                        WHERE id = $2
                        "#,
                        yield_calc.current_yield,
                        market.id
                    )
                    .execute(&self.pool)
                    .await
                    {
                        error!("Failed to update market {} yield: {}", market.id, e);
                        continue;
                    }

                    processed += 1;
                }
                Err(e) => {
                    error!("Failed to calculate yield for market {}: {}", market.id, e);
                }
            }
        }

        info!("Calculated yields for {} markets", processed);
        Ok(processed)
    }

    
    pub async fn get_yield_summary(&self) -> Result<YieldSummary> {
        let summary = sqlx::query!(
            r#"
            SELECT 
                p.name as protocol,
                p."displayName" as display_name,
                COALESCE(SUM(yr.amount), 0) as total_amount,
                COALESCE(SUM(yr.yield), 0) as total_yield,
                COALESCE(AVG(yr.apy), 0) as average_apy
            FROM protocols p
            LEFT JOIN yield_records yr ON p.id = yr."protocolId"
            WHERE p."isActive" = true
            GROUP BY p.id, p.name, p."displayName"
            ORDER BY total_yield DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut total_yield = BigDecimal::from(0);
        let mut protocol_breakdown = Vec::new();

        for row in summary {
            let protocol_yield = ProtocolSummary {
                protocol: row.protocol,
                display_name: row.display_name,
                total_amount: row.total_amount.unwrap_or(BigDecimal::from(0)),
                total_yield: row.total_yield.clone().unwrap_or(BigDecimal::from(0)),
                average_apy: row.average_apy.unwrap_or(BigDecimal::from(0)),
            };

            total_yield += row.total_yield.unwrap_or(BigDecimal::from(0));
            protocol_breakdown.push(protocol_yield);
        }

        Ok(YieldSummary {
            total_yield,
            protocol_breakdown,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct YieldSummary {
    pub total_yield: BigDecimal,
    pub protocol_breakdown: Vec<ProtocolSummary>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProtocolSummary {
    pub protocol: String,
    pub display_name: String,
    pub total_amount: BigDecimal,
    pub total_yield: BigDecimal,
    pub average_apy: BigDecimal,
}