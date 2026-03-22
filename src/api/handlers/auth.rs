use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::middleware::auth::AuthenticatedUser;
use crate::errors::AppError;
use crate::services::auth_service;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct NonceQuery {
    pub wallet: String,
}

#[derive(Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: String,
}

pub async fn get_nonce(
    State(state): State<AppState>,
    Query(q): Query<NonceQuery>,
) -> Result<Json<NonceResponse>, AppError> {
    let (nonce, expires_at) = auth_service::generate_nonce(&q.wallet, &state).await?;
    Ok(Json(NonceResponse {
        nonce,
        expires_at: expires_at.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub wallet: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user_id: String,
}

pub async fn verify_signature(
    State(state): State<AppState>,
    Json(body): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    let user = state
        .auth_store
        .get_user_by_wallet(&body.wallet.to_lowercase())
        .await?;

    let (access_token, refresh_token) =
        auth_service::verify_signature(&body.wallet, &body.signature, &state).await?;

    let updated_user = state
        .auth_store
        .upsert_user(&body.wallet.to_lowercase())
        .await?;

    Ok(Json(VerifyResponse {
        access_token,
        refresh_token,
        expires_in: state.config.jwt_expiry_secs,
        user_id: updated_user.user_id.to_string(),
    }))
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    let access_token = auth_service::refresh_token(&body.refresh_token, &state).await?;
    Ok(Json(RefreshResponse {
        access_token,
        expires_in: state.config.jwt_expiry_secs,
    }))
}

#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

pub async fn logout(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse>, AppError> {
    auth_service::logout(user.session_id, &state).await?;
    Ok(Json(SuccessResponse { success: true }))
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user_id: String,
    pub wallet: String,
    pub created_at: String,
}

pub async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<MeResponse>, AppError> {
    let u = state
        .auth_store
        .get_user_by_id(user.user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(MeResponse {
        user_id: u.user_id.to_string(),
        wallet: u.wallet,
        created_at: u.created_at.to_rfc3339(),
    }))
}
