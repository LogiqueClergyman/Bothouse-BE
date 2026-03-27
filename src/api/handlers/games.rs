use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::middleware::auth::AuthenticatedAgent;
use crate::domain::game::GameStatus;
use crate::errors::AppError;
use crate::services::game_service::{self, ActionRequest};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListGamesQuery {
    pub status: Option<String>,
    pub game_type: Option<String>,
    pub agent_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_games(
    State(state): State<AppState>,
    Query(q): Query<ListGamesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = q.status.as_deref().map(|s| match s {
        "waiting" => GameStatus::Waiting,
        "in_progress" => GameStatus::InProgress,
        "completed" => GameStatus::Completed,
        "cancelled" => GameStatus::Cancelled,
        _ => GameStatus::InProgress,
    });
    let games = state
        .game_store
        .list_games(status, q.game_type.as_deref(), q.limit.unwrap_or(20), q.offset.unwrap_or(0))
        .await?;

    // Omit current_state from list
    let sanitized: Vec<serde_json::Value> = games
        .into_iter()
        .map(|mut g| {
            let mut v = serde_json::to_value(&g).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.remove("current_state");
            }
            v
        })
        .collect();

    let total = sanitized.len();
    Ok(Json(serde_json::json!({ "games": sanitized, "total": total })))
}

pub async fn get_game(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let game = state
        .game_store
        .get_game_by_id(game_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut v = serde_json::to_value(&game).unwrap_or_default();
    if let Some(obj) = v.as_object_mut() {
        obj.remove("current_state");
    }
    Ok(Json(v))
}

pub async fn spectate_game(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let view = game_service::spectate_game(game_id, &state).await?;
    Ok(Json(view))
}

pub async fn get_game_state(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let view = game_service::get_game_state(game_id, agent.agent.agent_id, &state).await?;
    Ok(Json(view))
}

#[derive(Deserialize)]
pub struct ActionBody {
    pub action: String,
    pub amount_atomic: Option<String>,
    pub turn_number: i64,
    pub signature: String,
}

pub async fn submit_action(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(game_id): Path<Uuid>,
    Json(body): Json<ActionBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let req = ActionRequest {
        action: body.action,
        amount_atomic: body.amount_atomic,
        turn_number: body.turn_number,
        signature: body.signature,
    };
    let resp = game_service::submit_action(game_id, agent.agent.agent_id, req, &state).await?;
    Ok(Json(serde_json::json!({
        "accepted": resp.accepted,
        "sequence_number": resp.sequence_number,
    })))
}

pub async fn get_game_log(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let log = game_service::get_game_log(game_id, agent.agent.agent_id, &state).await?;
    Ok(Json(log))
}
