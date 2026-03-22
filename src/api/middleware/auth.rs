use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use uuid::Uuid;

use crate::domain::agent::Agent;
use crate::domain::auth::Claims;
use crate::errors::AppError;
use crate::services::agent_service;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub wallet: String,
    pub session_id: Uuid,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid Authorization header".to_string()))?;

        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized("UNAUTHORIZED".to_string()))?
        .claims;

        let user_id: Uuid = claims
            .sub
            .parse()
            .map_err(|_| AppError::Unauthorized("Invalid user_id in token".to_string()))?;

        let session_id: Uuid = claims
            .session_id
            .parse()
            .map_err(|_| AppError::Unauthorized("Invalid session_id in token".to_string()))?;

        Ok(AuthenticatedUser {
            user_id,
            wallet: claims.wallet,
            session_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedAgent {
    pub agent: Agent,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthenticatedAgent {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .headers
            .get("X-Agent-Key")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing X-Agent-Key header".to_string()))?;

        let agent = agent_service::authenticate_agent_key(api_key, state).await?;
        Ok(AuthenticatedAgent { agent })
    }
}

/// Optional auth — returns None if not authenticated, doesn't fail.
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<AuthenticatedUser>);

#[axum::async_trait]
impl FromRequestParts<AppState> for OptionalUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(OptionalUser(
            AuthenticatedUser::from_request_parts(parts, state)
                .await
                .ok(),
        ))
    }
}
