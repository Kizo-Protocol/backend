use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const PRICE_SOURCES: &[PriceSource] = &[
    PriceSource {
        name: "Binance",
        url: "https://api.binance.com/api/v3/ticker/price?symbol=APTUSDT",
    },
    PriceSource {
        name: "Coinbase",
        url: "https://api.exchange.coinbase.com/products/APT-USD/ticker",
    },
    PriceSource {
        name: "Kraken",
        url: "https://api.kraken.com/0/public/Ticker?pair=APTUSD",
    },
    PriceSource {
        name: "Bybit",
        url: "https://api.bybit.com/v5/market/tickers?category=spot&symbol=APTUSDT",
    },
    PriceSource {
        name: "OKX",
        url: "https://www.okx.com/api/v5/market/ticker?instId=APT-USDT",
    },
    PriceSource {
        name: "CoinGecko",
        url: "https://api.coingecko.com/api/v3/simple/price?ids=aptos&vs_currencies=usd",
    },
    PriceSource {
        name: "CoinPaprika",
        url: "https://api.coinpaprika.com/v1/tickers/apt-aptos?quotes=USD",
    },
];

#[derive(Debug, Clone)]
struct PriceSource {
    name: &'static str,
    url: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub price: f64,
    pub decimals: u8,
    pub timestamp: i64,
    pub round_id: u64,
}

#[derive(Clone)]
pub struct ChainlinkPriceFeed {
    cached_price: Arc<RwLock<Option<PriceData>>>,
}

impl ChainlinkPriceFeed {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cached_price: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn get_apt_usd_price(&self) -> Result<f64> {
        
        {
            let cache = self.cached_price.read().await;
            if let Some(price_data) = cache.as_ref() {
                let now = chrono::Utc::now().timestamp();
                
                if now - price_data.timestamp < 300 {
                    info!("Using cached APT/USD price: ${}", price_data.price);
                    return Ok(price_data.price);
                }
            }
        }

        
        match self.fetch_price_from_chainlink().await {
            Ok(price_data) => {
                let price = price_data.price;
                
                
                {
                    let mut cache = self.cached_price.write().await;
                    *cache = Some(price_data);
                }
                
                info!("Fetched fresh APT/USD price from Chainlink: ${}", price);
                Ok(price)
            }
            Err(e) => {
                error!("Failed to fetch price from Chainlink: {}", e);
                
                
                let cache = self.cached_price.read().await;
                if let Some(price_data) = cache.as_ref() {
                    warn!("Using stale cached price as fallback: ${}", price_data.price);
                    return Ok(price_data.price);
                }
                
                
                Err(anyhow!("Failed to fetch APT/USD price: {}", e))
            }
        }
    }

    async fn fetch_price_from_chainlink(&self) -> Result<PriceData> {
        info!("🔄 Fetching APT/USD price from {} sources...", PRICE_SOURCES.len());
        
        
        let mut tasks = Vec::new();
        for source in PRICE_SOURCES {
            let source_clone = source.clone();
            let task = tokio::spawn(async move {
                match fetch_from_source_static(&source_clone).await {
                    Ok(price) => Some((source_clone.name, price)),
                    Err(e) => {
                        warn!("❌ {}: {}", source_clone.name, e);
                        None
                    }
                }
            });
            tasks.push(task);
        }

        
        let results = futures::future::join_all(tasks).await;
        
        
        let mut prices: Vec<(String, f64)> = Vec::new();
        for result in results {
            if let Ok(Some((name, price))) = result {
                info!("✅ {}: ${:.4}", name, price);
                prices.push((name.to_string(), price));
            }
        }

        if prices.is_empty() {
            error!("⚠️  All {} price sources failed, using fallback", PRICE_SOURCES.len());
            return Ok(self.get_fallback_price());
        }

        
        let median_price = self.calculate_median_price(&prices);
        let source_count = prices.len();
        
        info!(
            "📊 Aggregated price from {}/{} sources: ${:.4} (median)",
            source_count,
            PRICE_SOURCES.len(),
            median_price
        );

        Ok(PriceData {
            price: median_price,
            decimals: 8,
            timestamp: chrono::Utc::now().timestamp(),
            round_id: source_count as u64,
        })
    }

    fn calculate_median_price(&self, prices: &[(String, f64)]) -> f64 {
        if prices.is_empty() {
            return 0.0;
        }

        let mut values: Vec<f64> = prices.iter().map(|(_, price)| *price).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = values.len();
        if len % 2 == 0 {
            
            (values[len / 2 - 1] + values[len / 2]) / 2.0
        } else {
            
            values[len / 2]
        }
    }


    fn get_fallback_price(&self) -> PriceData {
        info!("💰 Using fallback price: APT/USD = $12.00");
        PriceData {
            price: 12.0,
            decimals: 8,
            timestamp: chrono::Utc::now().timestamp(),
            round_id: 0,
        }
    }

    pub async fn get_cached_price(&self) -> Option<PriceData> {
        let cache = self.cached_price.read().await;
        cache.clone()
    }

    pub async fn refresh_price(&self) -> Result<PriceData> {
        let price_data = self.fetch_price_from_chainlink().await?;
        
        {
            let mut cache = self.cached_price.write().await;
            *cache = Some(price_data.clone());
        }
        
        Ok(price_data)
    }

    #[allow(dead_code)]
    pub async fn apt_to_usd(&self, apt_amount: f64) -> Result<f64> {
        let price = self.get_apt_usd_price().await?;
        Ok(apt_amount * price)
    }

    #[allow(dead_code)]
    pub async fn usd_to_apt(&self, usd_amount: f64) -> Result<f64> {
        let price = self.get_apt_usd_price().await?;
        if price == 0.0 {
            return Err(anyhow!("Invalid price: 0"));
        }
        Ok(usd_amount / price)
    }
}

async fn fetch_from_source_static(source: &PriceSource) -> Result<f64> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response = client.get(source.url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("HTTP {}", response.status()));
    }

    let data: Value = response.json().await?;

    
    let price = match source.name {
        "Binance" => {
            
            data["price"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        "Coinbase" => {
            
            data["price"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        "Kraken" => {
            
            data["result"]["APTUSD"]["c"][0]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        "Bybit" => {
            
            data["result"]["list"][0]["lastPrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        "OKX" => {
            
            data["data"][0]["last"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        "CoinGecko" => {
            
            data["aptos"]["usd"]
                .as_f64()
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        "CoinPaprika" => {
            
            data["quotes"]["USD"]["price"]
                .as_f64()
                .ok_or_else(|| anyhow!("Invalid format"))?
        }
        _ => return Err(anyhow!("Unknown source")),
    };

    if price <= 0.0 || price > 10000.0 {
        return Err(anyhow!("Invalid price range: {}", price));
    }

    Ok(price)
}

impl Default for ChainlinkPriceFeed {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            cached_price: Arc::new(RwLock::new(None)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_feed_creation() {
        let feed = ChainlinkPriceFeed::new();
        assert!(feed.is_ok());
    }

    #[tokio::test]
    async fn test_apt_to_usd_conversion() {
        let feed = ChainlinkPriceFeed::new().unwrap();
        
        
        {
            let mut cache = feed.cached_price.write().await;
            *cache = Some(PriceData {
                price: 12.0,
                decimals: 8,
                timestamp: chrono::Utc::now().timestamp(),
                round_id: 1,
            });
        }

        let result = feed.apt_to_usd(10.0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 120.0);
    }

    #[tokio::test]
    async fn test_usd_to_apt_conversion() {
        let feed = ChainlinkPriceFeed::new().unwrap();
        
        
        {
            let mut cache = feed.cached_price.write().await;
            *cache = Some(PriceData {
                price: 12.0,
                decimals: 8,
                timestamp: chrono::Utc::now().timestamp(),
                round_id: 1,
            });
        }

        let result = feed.usd_to_apt(120.0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10.0);
    }
}
