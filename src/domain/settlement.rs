use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "settlement_status", rename_all = "lowercase")]
pub enum SettlementStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub settlement_id: Uuid,
    pub game_id: Uuid,
    pub status: SettlementStatus,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub retry_count: i16,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
