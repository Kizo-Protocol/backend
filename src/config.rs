use anyhow::{Context, Result};
use std::env;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub cors_origin: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3002".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;

        let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        let cors_origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string());

        Ok(Self {
            database_url,
            host,
            port,
            log_level,
            cors_origin,
        })
    }
}
