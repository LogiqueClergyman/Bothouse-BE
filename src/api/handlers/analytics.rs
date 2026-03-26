use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::metrics_service;
use crate::state::AppState;

fn analytics_headers(sample_size: i64, computed_at: &chrono::DateTime<chrono::Utc>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Sample-Size",
        HeaderValue::from_str(&sample_size.to_string()).unwrap_or(HeaderValue::from_static("0")),
    );
    headers.insert(
        "X-Computed-At",
        HeaderValue::from_str(&computed_at.to_rfc3339())
            .unwrap_or(HeaderValue::from_static("")),
    );
    headers
}

#[derive(Deserialize)]
pub struct TendenciesQuery {
    pub game_type: Option<String>,
}

/// GET /agents/:agent_id/tendencies
pub async fn get_tendencies(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Query(q): Query<TendenciesQuery>,
) -> Result<impl IntoResponse, AppError> {
    let tendencies = metrics_service::get_tendencies(agent_id, q.game_type, &state).await?;
    let headers = analytics_headers(tendencies.sample_size, &tendencies.computed_at);
    let body = serde_json::to_value(&tendencies).unwrap_or_default();
    Ok((headers, Json(body)))
}

#[derive(Deserialize)]
pub struct ActionQuery {
    pub game_type: Option<String>,
    pub game_id: Option<String>,
    pub phase: Option<String>,
    pub action: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order: Option<String>,
}

/// GET /agents/:agent_id/actions
pub async fn list_actions(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Query(q): Query<ActionQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0);
    let actions =
        metrics_service::list_actions(agent_id, q.game_type, limit, offset, &state).await?;
    let total = actions.len() as i64; // simplified; full impl would do a COUNT query
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "actions": actions,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

#[derive(Deserialize)]
pub struct HandQuery {
    pub game_type: Option<String>,
    pub game_id: Option<String>,
    pub result: Option<String>,
    pub went_to_showdown: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /agents/:agent_id/hands
pub async fn list_hands(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Query(q): Query<HandQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0);
    let hands = metrics_service::list_hands(agent_id, q.game_type, limit, offset, &state).await?;
    let total = hands.len() as i64;
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "hands": hands,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

#[derive(Deserialize)]
pub struct H2HQuery {
    pub game_type: Option<String>,
}

/// GET /agents/:agent_id/vs/:opponent_id
pub async fn get_head_to_head(
    State(state): State<AppState>,
    Path((agent_id, opponent_id)): Path<(Uuid, Uuid)>,
    Query(q): Query<H2HQuery>,
) -> Result<impl IntoResponse, AppError> {
    let record =
        metrics_service::get_head_to_head(agent_id, opponent_id, q.game_type, &state).await?;
    let headers = analytics_headers(record.hands_together as i64, &record.computed_at);
    let body = serde_json::to_value(&record).unwrap_or_default();
    Ok((headers, Json(body)))
}
