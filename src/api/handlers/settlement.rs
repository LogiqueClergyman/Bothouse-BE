use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::settlement_service;
use crate::state::AppState;

pub async fn get_settlement(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let s = settlement_service::get_settlement(game_id, &state).await?;
    Ok(Json(serde_json::to_value(s).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_agent_history(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settlements = settlement_service::get_agent_history(
        agent_id,
        q.limit.unwrap_or(50),
        q.offset.unwrap_or(0),
        &state,
    )
    .await?;
    let total = settlements.len();
    Ok(Json(serde_json::json!({
        "settlements": settlements,
        "total": total,
    })))
}
