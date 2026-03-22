use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::middleware::auth::{AuthenticatedUser, OptionalUser};
use crate::domain::agent::AgentStatus;
use crate::errors::AppError;
use crate::services::agent_service::{
    self, RegisterAgentRequest, UpdateAgentRequest,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterAgentBody {
    pub name: String,
    pub wallet_address: String,
    pub description: Option<String>,
    pub webhook_url: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterAgentResponse {
    pub agent_id: String,
    pub api_key: String,
    pub wallet_address: String,
    pub name: String,
    pub created_at: String,
}

pub async fn register_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<RegisterAgentBody>,
) -> Result<impl IntoResponse, AppError> {
    let req = RegisterAgentRequest {
        name: body.name,
        wallet_address: body.wallet_address,
        description: body.description,
        webhook_url: body.webhook_url,
    };
    let (agent, raw_key) = agent_service::register_agent(user.user_id, req, &state).await?;
    Ok((
        StatusCode::CREATED,
        Json(RegisterAgentResponse {
            agent_id: agent.agent_id.to_string(),
            api_key: raw_key,
            wallet_address: agent.wallet_address,
            name: agent.name,
            created_at: agent.created_at.to_rfc3339(),
        }),
    ))
}

#[derive(Serialize)]
pub struct AgentsResponse {
    pub agents: Vec<serde_json::Value>,
}

pub async fn list_agents(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<AgentsResponse>, AppError> {
    let agents = agent_service::list_agents(user.user_id, &state).await?;
    let values = agents
        .into_iter()
        .map(|a| serde_json::to_value(a).unwrap_or_default())
        .collect();
    Ok(Json(AgentsResponse { agents: values }))
}

pub async fn get_agent(
    State(state): State<AppState>,
    OptionalUser(opt_user): OptionalUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let requesting_user_id = opt_user.map(|u| u.user_id);
    let view = agent_service::get_agent(agent_id, requesting_user_id, &state).await?;
    Ok(Json(serde_json::to_value(view).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct UpdateAgentBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub webhook_url: Option<String>,
    pub status: Option<String>,
}

pub async fn update_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<UpdateAgentBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = body.status.as_deref().map(|s| match s {
        "active" => AgentStatus::Active,
        "paused" => AgentStatus::Paused,
        _ => AgentStatus::Active,
    });
    let req = UpdateAgentRequest {
        name: body.name,
        description: body.description,
        webhook_url: body.webhook_url,
        status,
    };
    let agent = agent_service::update_agent(agent_id, user.user_id, req, &state).await?;
    Ok(Json(serde_json::to_value(agent).unwrap_or_default()))
}

pub async fn rotate_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let raw_key = agent_service::rotate_api_key(agent_id, user.user_id, &state).await?;
    Ok(Json(serde_json::json!({ "api_key": raw_key })))
}

pub async fn get_stats(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = agent_service::get_stats(agent_id, &state).await?;
    Ok(Json(serde_json::json!({ "stats": stats })))
}

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    pub game_type: Option<String>,
    pub sort_by: Option<String>,
    pub period: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_leaderboard(
    State(state): State<AppState>,
    Query(q): Query<LeaderboardQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sort_by = q.sort_by.unwrap_or_else(|| "net_profit_wei".to_string());
    let period = q.period.unwrap_or_else(|| "all_time".to_string());
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);

    let entries = agent_service::get_leaderboard(
        q.game_type,
        sort_by,
        period,
        limit,
        offset,
        &state,
    )
    .await?;

    let total = entries.len();
    Ok(Json(serde_json::json!({
        "leaderboard": entries,
        "total": total,
    })))
}
