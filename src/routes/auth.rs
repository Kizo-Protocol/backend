use axum::{
    extract::State,
    middleware,
    response::Json,
    routing::{get, post, put},
    Extension,
    Router,
};
use serde_json::json;

use crate::{
    db::Database,
    error::AppError,
    middleware::jwt::require_jwt,
    models::{
        UpdateProfileRequest, UpdateProfileResponse, UpdateProfileData,
        WalletConnectRequest, WalletConnectResponse, WalletConnectData,
    },
    services::user_service::UserService,
    utils::jwt::{Claims, JwtService},
};

pub fn create_auth_router() -> Router<Database> {
    
    let public_routes = Router::new()
        .route("/wallet", post(connect_wallet));
    
    
    let protected_routes = Router::new()
        .route("/me", get(get_current_user))
        .route("/profile", put(update_profile))
        .route("/refresh", post(refresh_token))
        .route_layer(middleware::from_fn(require_jwt));
    
    
    public_routes.merge(protected_routes)
}

#[utoipa::path(
    post,
    path = "/api/auth/wallet",
    request_body = WalletConnectRequest,
    responses(
        (status = 200, description = "Successfully connected wallet", body = WalletConnectResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth"
)]
async fn connect_wallet(
    State(db): State<Database>,
    Json(payload): Json<WalletConnectRequest>,
) -> Result<Json<WalletConnectResponse>, AppError> {
    let user_service = UserService::new(db.pool().clone());
    let jwt_service = JwtService::new();

    
    if payload.address.is_empty() {
        return Err(AppError::BadRequest("Address cannot be empty".to_string()));
    }

    
    let user = user_service.get_or_create_user(&payload.address).await?;

    
    let token = jwt_service
        .generate_token(user.id.clone(), user.address.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to generate token: {}", e)))?;

    Ok(Json(WalletConnectResponse {
        message: "Successfully connected wallet".to_string(),
        data: WalletConnectData { user, token },
    }))
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Successfully retrieved user data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "auth"
)]
async fn get_current_user(
    State(db): State<Database>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {

    let user_service = UserService::new(db.pool().clone());
    
    
    let user_with_bets = user_service.get_user_with_bets(&claims.sub).await?;

    Ok(Json(json!({
        "data": user_with_bets
    })))
}

#[utoipa::path(
    put,
    path = "/api/auth/profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Successfully updated profile", body = UpdateProfileResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "auth"
)]
async fn update_profile(
    State(db): State<Database>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UpdateProfileResponse>, AppError> {

    let user_service = UserService::new(db.pool().clone());

    
    let user = user_service
        .update_profile(&claims.sub, payload.username, payload.avatar_url)
        .await?;

    Ok(Json(UpdateProfileResponse {
        message: "Profile updated successfully".to_string(),
        data: UpdateProfileData { user },
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    responses(
        (status = 200, description = "Successfully refreshed token"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "auth"
)]
async fn refresh_token(
    State(db): State<Database>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {

    let user_service = UserService::new(db.pool().clone());
    let jwt_service = JwtService::new();

    
    let user = user_service
        .get_user_by_id(&claims.sub)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    
    let token = jwt_service
        .generate_token(user.id.clone(), user.address.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to generate token: {}", e)))?;

    Ok(Json(json!({
        "message": "Token refreshed successfully",
        "data": {
            "token": token
        }
    })))
}
