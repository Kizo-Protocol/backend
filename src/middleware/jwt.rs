use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::utils::jwt::JwtService;

pub async fn require_jwt(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let jwt_service = JwtService::new();

    
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "Unauthorized",
                    "message": "Missing authorization header"
                })),
            )
                .into_response()
        })?;

    
    let token = JwtService::extract_token_from_header(auth_header).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "error": "Unauthorized",
                "message": "Invalid authorization header format. Use 'Bearer <token>'"
            })),
        )
            .into_response()
    })?;

    
    let claims = jwt_service.validate_token(token).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "error": "Unauthorized",
                "message": format!("Invalid token: {}", e)
            })),
        )
            .into_response()
    })?;

    
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

