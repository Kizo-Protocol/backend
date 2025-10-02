use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub address: String,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn new(user_id: String, address: String, expires_in_hours: i64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as usize;

        let exp = now + (expires_in_hours * 3600) as usize;

        Self {
            sub: user_id,
            address,
            exp,
            iat: now,
        }
    }
}

pub struct JwtService {
    secret: String,
}

impl JwtService {
    pub fn new() -> Self {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

        if secret == "your-secret-key-change-in-production" {
            tracing::warn!(
                "⚠️  Using default JWT_SECRET! Please set JWT_SECRET environment variable in production"
            );
        }

        Self { secret }
    }

    pub fn generate_token(&self, user_id: String, address: String) -> Result<String, String> {
        let claims = Claims::new(user_id, address, 24 * 7);

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| format!("Failed to generate token: {}", e))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, String> {
        let validation = Validation::new(Algorithm::HS256);

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| format!("Invalid token: {}", e))
    }

    pub fn extract_token_from_header(auth_header: &str) -> Option<&str> {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            Some(token)
        } else {
            None
        }
    }
}

impl Default for JwtService {
    fn default() -> Self {
        Self::new()
    }
}
