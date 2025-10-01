use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub async fn require_api_key(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    
    let valid_api_key = std::env::var("API_KEY").unwrap_or_else(|_| "".to_string());
    
    
    if valid_api_key.is_empty() {
        tracing::warn!("API_KEY not configured - allowing all write operations (NOT SECURE FOR PRODUCTION)");
        return Ok(next.run(request).await);
    }
    
    
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_value) = auth_header.to_str() {
            
            let token = auth_value
                .strip_prefix("Bearer ")
                .unwrap_or(auth_value)
                .trim();
            
            if token == valid_api_key {
                return Ok(next.run(request).await);
            }
        }
    }
    
    
    if let Some(api_key_header) = headers.get("X-API-Key") {
        if let Ok(key) = api_key_header.to_str() {
            if key == valid_api_key {
                return Ok(next.run(request).await);
            }
        }
    }
    
    
    Err((
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({
            "error": "Unauthorized",
            "message": "Valid API key required for this operation. Provide it via 'Authorization: Bearer YOUR_KEY' or 'X-API-Key: YOUR_KEY' header"
        })),
    )
        .into_response())
}
