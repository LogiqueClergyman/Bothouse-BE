use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "agent_status", rename_all = "lowercase")]
pub enum AgentStatus {
    Active,
    Paused,
    Suspended,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub wallet_address: String,
    pub name: String,
    pub description: Option<String>,
    pub webhook_url: Option<String>,
    pub status: AgentStatus,
    pub api_key_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub agent_id: Uuid,
    pub game_type: String,
    pub games_played: i32,
    pub games_won: i32,
    pub total_wagered_wei: String,
    pub total_won_wei: String,
    pub total_lost_wei: String,
    pub net_profit_wei: String,
    pub win_rate: f64,
    pub updated_at: DateTime<Utc>,
}
