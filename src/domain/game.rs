use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::DomainError;

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "game_status", rename_all = "lowercase")]
pub enum GameStatus {
    Waiting,
    #[sqlx(rename = "in_progress")]
    InProgress,
    Completed,
    Cancelled,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "player_status", rename_all = "lowercase")]
pub enum PlayerStatus {
    Active,
    Folded,
    #[sqlx(rename = "all_in")]
    AllIn,
    Busted,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInstance {
    pub game_id: Uuid,
    pub room_id: Uuid,
    pub game_type: String,
    pub game_version: String,
    pub status: GameStatus,
    pub current_state: serde_json::Value,
    pub sequence_number: i64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayer {
    pub game_id: Uuid,
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub seat_number: i16,
    pub stack_atomic: String,
    pub status: PlayerStatus,
    pub consecutive_timeouts: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLogEntry {
    pub game_id: Uuid,
    pub sequence: i64,
    pub timestamp: DateTime<Utc>,
    pub agent_id: Option<Uuid>,
    pub action: String,
    pub amount_atomic: Option<String>,
    pub state_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    pub game_id: Uuid,
    pub winners: Vec<WinnerEntry>,
    pub losers: Vec<LoserEntry>,
    pub rake_atomic: String,
    pub rake_rate_bps: i16,
    pub signed_result_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinnerEntry {
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub amount_won_atomic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoserEntry {
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub amount_lost_atomic: String,
}

pub trait Game: Send + Sync {
    fn game_type(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn min_players(&self) -> usize;
    fn max_players(&self) -> usize;
    fn turn_timeout_ms(&self) -> u64;

    fn init(
        &self,
        players: Vec<GamePlayer>,
        seed: [u8; 32],
    ) -> Result<serde_json::Value, DomainError>;

    fn visible_state(
        &self,
        state: &serde_json::Value,
        agent_id: Uuid,
    ) -> Result<serde_json::Value, DomainError>;

    fn valid_actions(
        &self,
        state: &serde_json::Value,
        agent_id: Uuid,
    ) -> Result<Vec<String>, DomainError>;

    fn apply_action(
        &self,
        state: serde_json::Value,
        agent_id: Uuid,
        action: &str,
        amount_atomic: Option<&str>,
    ) -> Result<serde_json::Value, DomainError>;

    fn is_terminal(&self, state: &serde_json::Value) -> bool;

    fn result(
        &self,
        state: &serde_json::Value,
        game_id: Uuid,
        rake_bps: u16,
    ) -> Option<GameResult>;

    fn timeout_action(&self, state: &serde_json::Value, agent_id: Uuid) -> String;

    fn verify_action_signature(
        &self,
        game_id: Uuid,
        turn_number: i64,
        action: &str,
        amount_atomic: Option<&str>,
        signature: &str,
        wallet_address: &str,
        chain_type: &str,
    ) -> Result<bool, DomainError>;
}
