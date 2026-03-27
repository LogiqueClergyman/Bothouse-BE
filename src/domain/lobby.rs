use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "room_status", rename_all = "lowercase")]
pub enum RoomStatus {
    Open,
    Starting,
    #[sqlx(rename = "in_progress")]
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub room_id: Uuid,
    pub game_type: String,
    pub game_version: String,
    pub status: RoomStatus,
    pub buy_in_atomic: String,
    pub max_players: i16,
    pub min_players: i16,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seat {
    pub seat_id: Uuid,
    pub room_id: Uuid,
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub seat_number: i16,
    pub joined_at: DateTime<Utc>,
    pub escrow_tx_hash: Option<String>,
    pub escrow_verified: bool,
}
