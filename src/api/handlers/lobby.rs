use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::middleware::auth::AuthenticatedAgent;
use crate::domain::lobby::RoomStatus;
use crate::errors::AppError;
use crate::services::lobby_service::{
    self, CreateRoomRequest, JoinQueueRequest, RoomFilters,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListRoomsQuery {
    pub game_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_rooms(
    State(state): State<AppState>,
    Query(q): Query<ListRoomsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = q.status.as_deref().map(|s| match s {
        "open" => RoomStatus::Open,
        "in_progress" => RoomStatus::InProgress,
        "completed" => RoomStatus::Completed,
        "cancelled" => RoomStatus::Cancelled,
        _ => RoomStatus::Open,
    });
    let filters = RoomFilters {
        game_type: q.game_type,
        status,
        limit: q.limit.unwrap_or(20).min(100),
        offset: q.offset.unwrap_or(0),
    };
    let rooms = lobby_service::list_rooms(filters, &state).await?;
    let total = rooms.len();
    Ok(Json(serde_json::json!({
        "rooms": rooms,
        "total": total,
    })))
}

pub async fn get_room(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let room = lobby_service::get_room(room_id, &state).await?;
    Ok(Json(serde_json::to_value(room).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct CreateRoomBody {
    pub game_type: String,
    pub buy_in_wei: String,
    pub max_players: i16,
    pub min_players: i16,
    pub escrow_tx_hash: String,
}

pub async fn create_room(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(body): Json<CreateRoomBody>,
) -> Result<impl IntoResponse, AppError> {
    let req = CreateRoomRequest {
        game_type: body.game_type,
        buy_in_wei: body.buy_in_wei,
        max_players: body.max_players,
        min_players: body.min_players,
        escrow_tx_hash: body.escrow_tx_hash,
    };
    let room = lobby_service::create_room(agent.agent.agent_id, req, &state).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(room).unwrap_or_default())))
}

#[derive(Deserialize)]
pub struct JoinRoomBody {
    pub escrow_tx_hash: String,
}

pub async fn join_room(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(room_id): Path<Uuid>,
    Json(body): Json<JoinRoomBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let seat = lobby_service::join_room(
        agent.agent.agent_id,
        room_id,
        &body.escrow_tx_hash,
        &state,
    )
    .await?;
    let room = lobby_service::get_room(room_id, &state).await?;
    Ok(Json(serde_json::json!({
        "seat_number": seat.seat_number,
        "room": room,
    })))
}

pub async fn leave_room(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(room_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    lobby_service::leave_room(agent.agent.agent_id, room_id, &state).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct JoinQueueBody {
    pub game_type: String,
    pub buy_in_wei: String,
    pub max_players: i16,
    pub escrow_tx_hash: String,
}

pub async fn join_queue(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(body): Json<JoinQueueBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let req = JoinQueueRequest {
        game_type: body.game_type,
        buy_in_wei: body.buy_in_wei,
        max_players: body.max_players,
        escrow_tx_hash: body.escrow_tx_hash,
    };
    let resp = lobby_service::join_queue(agent.agent.agent_id, req, &state).await?;
    Ok(Json(serde_json::json!({
        "room_id": resp.room_id.to_string(),
        "seat_number": resp.seat_number,
        "status": resp.status,
    })))
}
