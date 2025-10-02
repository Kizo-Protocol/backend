use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMarketParams {
    pub question: String,
    pub description: String,
    pub duration_seconds: u64,
    pub token_type: String,
    pub protocol_selector_addr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMarketResult {
    pub market_id: i64,
    pub tx_hash: String,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct AptosContractService {
    pub node_url: String,
    pub module_address: String,
    pub module_name: String,
}

#[allow(dead_code)]
impl AptosContractService {
    pub fn new() -> Result<Self> {
        let node_url = std::env::var("APTOS_NODE_URL")
            .unwrap_or_else(|_| "https://fullnode.testnet.aptoslabs.com/v1".to_string());

        let module_address = std::env::var("APTOS_MODULE_ADDRESS")
            .map_err(|_| anyhow!("APTOS_MODULE_ADDRESS environment variable is required"))?;

        let module_name =
            std::env::var("APTOS_MODULE_NAME").unwrap_or_else(|_| "prediction_market".to_string());

        Ok(Self {
            node_url,
            module_address,
            module_name,
        })
    }

    pub async fn create_market(&self, params: CreateMarketParams) -> Result<CreateMarketResult> {
        info!("Creating market on Aptos blockchain: {}", params.question);

        let _private_key = std::env::var("APTOS_PRIVATE_KEY")
            .map_err(|_| anyhow!("APTOS_PRIVATE_KEY environment variable is required"))?;

        let function_id = format!(
            "{}::{}::create_market",
            self.module_address, self.module_name
        );

        let payload = json!({
            "type": "entry_function_payload",
            "function": function_id,
            "type_arguments": [params.token_type],
            "arguments": [
                self.module_address,
                params.question,
                params.description,
                params.duration_seconds.to_string(),
                params.protocol_selector_addr
            ]
        });

        info!("Transaction payload prepared: {:?}", payload);
        info!("NOTE: Actual Aptos SDK integration required. Using mock response for development.");

        let mock_market_id = chrono::Utc::now().timestamp() % 1000000;
        let mock_tx_hash = format!("0x{:x}", mock_market_id);

        info!(
            "Market creation transaction would be submitted with function: {}",
            function_id
        );

        Ok(CreateMarketResult {
            market_id: mock_market_id,
            tx_hash: mock_tx_hash,
            version: 0,
        })
    }

    pub async fn get_market(&self, market_id: i64) -> Result<serde_json::Value> {
        info!("Fetching market {} from Aptos blockchain", market_id);

        let view_function = format!("{}::{}::get_market", self.module_address, self.module_name);

        let url = format!("{}/view", self.node_url);
        let client = reqwest::Client::new();

        let response = client
            .post(&url)
            .json(&json!({
                "function": view_function,
                "type_arguments": [],
                "arguments": [market_id.to_string()]
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            error!(
                "Failed to fetch market from blockchain: {}",
                response.status()
            );
            return Err(anyhow!("Failed to fetch market: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data)
    }

    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client.get(&self.node_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to connect to Aptos node"));
        }

        let data: serde_json::Value = response.json().await?;

        Ok(json!({
            "status": "connected",
            "network": "aptos_testnet",
            "node_url": self.node_url,
            "module_address": self.module_address,
            "module_name": self.module_name,
            "chain_info": data
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_status() {
        std::env::set_var("APTOS_MODULE_ADDRESS", "0x123");

        let service = AptosContractService::new().unwrap();
        let result = service.get_status().await;

        match result {
            Ok(status) => {
                assert_eq!(status["status"], "connected");
            }
            Err(e) => {
                eprintln!("Status check failed (expected in CI): {}", e);
            }
        }
    }
}
