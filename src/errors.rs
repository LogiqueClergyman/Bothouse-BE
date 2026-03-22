use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::domain::DomainError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            AppError::Unauthorized(msg) => {
                let code = if msg.contains("NONCE_EXPIRED") {
                    "NONCE_EXPIRED"
                } else if msg.contains("INVALID_REFRESH_TOKEN") {
                    "INVALID_REFRESH_TOKEN"
                } else if msg.contains("INVALID_SIGNATURE") {
                    "INVALID_SIGNATURE"
                } else {
                    "UNAUTHORIZED"
                };
                (StatusCode::UNAUTHORIZED, code, msg.clone())
            }
            AppError::Forbidden(msg) => {
                let code = if msg.contains("NOT_YOUR_TURN") {
                    "NOT_YOUR_TURN"
                } else if msg.contains("GAME_ALREADY_STARTED") {
                    "GAME_ALREADY_STARTED"
                } else if msg.contains("AGENT_SUSPENDED") {
                    "AGENT_SUSPENDED"
                } else {
                    "FORBIDDEN"
                };
                (StatusCode::FORBIDDEN, code, msg.clone())
            }
            AppError::BadRequest(msg) => {
                let code = if msg.contains("ESCROW_NOT_VERIFIED") {
                    "ESCROW_NOT_VERIFIED"
                } else if msg.contains("INVALID_ACTION") {
                    "INVALID_ACTION"
                } else if msg.contains("INVALID_AMOUNT") {
                    "INVALID_AMOUNT"
                } else {
                    "BAD_REQUEST"
                };
                (StatusCode::BAD_REQUEST, code, msg.clone())
            }
            AppError::Conflict(msg) => {
                let code = if msg.contains("ROOM_FULL") {
                    "ROOM_FULL"
                } else if msg.contains("ROOM_NOT_OPEN") {
                    "ROOM_NOT_OPEN"
                } else if msg.contains("GAME_NOT_IN_PROGRESS") {
                    "GAME_NOT_IN_PROGRESS"
                } else {
                    "CONFLICT"
                };
                (StatusCode::CONFLICT, code, msg.clone())
            }
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An unexpected error occurred".to_string(),
            ),
            AppError::Domain(e) => match e {
                DomainError::InvalidAction(msg) => (
                    StatusCode::BAD_REQUEST,
                    "INVALID_ACTION",
                    msg.clone(),
                ),
                DomainError::InvalidAmount(msg) => (
                    StatusCode::BAD_REQUEST,
                    "INVALID_AMOUNT",
                    msg.clone(),
                ),
                DomainError::NotYourTurn => (
                    StatusCode::FORBIDDEN,
                    "NOT_YOUR_TURN",
                    "It is not your turn".to_string(),
                ),
                DomainError::GameNotInProgress => (
                    StatusCode::CONFLICT,
                    "GAME_NOT_IN_PROGRESS",
                    "Game is not in progress".to_string(),
                ),
                DomainError::InvalidSignature => (
                    StatusCode::UNAUTHORIZED,
                    "INVALID_SIGNATURE",
                    "Invalid signature".to_string(),
                ),
                DomainError::StateParseError(msg) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    msg.clone(),
                ),
            },
        };

        let body = Json(json!({
            "error": code,
            "message": message,
        }));

        (status, body).into_response()
    }
}
