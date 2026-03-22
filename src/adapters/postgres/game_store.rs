use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::game::{
    GameInstance, GameLogEntry, GamePlayer, GameResult, GameStatus, LoserEntry, PlayerStatus,
    WinnerEntry,
};
use crate::errors::AppError;
use crate::ports::game_store::GameStore;

pub struct PgGameStore {
    pool: PgPool,
}

impl PgGameStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GameRow {
    game_id: Uuid,
    room_id: Uuid,
    game_type: String,
    game_version: String,
    status: GameStatus,
    current_state: serde_json::Value,
    sequence_number: i64,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<GameRow> for GameInstance {
    fn from(row: GameRow) -> Self {
        Self {
            game_id: row.game_id,
            room_id: row.room_id,
            game_type: row.game_type,
            game_version: row.game_version,
            status: row.status,
            current_state: row.current_state,
            sequence_number: row.sequence_number,
            created_at: row.created_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GamePlayerRow {
    game_id: Uuid,
    agent_id: Uuid,
    wallet_address: String,
    seat_number: i16,
    stack_wei: sqlx::types::BigDecimal,
    status: PlayerStatus,
    consecutive_timeouts: i16,
}

impl From<GamePlayerRow> for GamePlayer {
    fn from(row: GamePlayerRow) -> Self {
        Self {
            game_id: row.game_id,
            agent_id: row.agent_id,
            wallet_address: row.wallet_address,
            seat_number: row.seat_number,
            stack_wei: row.stack_wei.to_string(),
            status: row.status,
            consecutive_timeouts: row.consecutive_timeouts,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GameLogRow {
    game_id: Uuid,
    sequence: i64,
    timestamp: DateTime<Utc>,
    agent_id: Option<Uuid>,
    action: String,
    amount_wei: Option<sqlx::types::BigDecimal>,
    state_hash: String,
}

impl From<GameLogRow> for GameLogEntry {
    fn from(row: GameLogRow) -> Self {
        Self {
            game_id: row.game_id,
            sequence: row.sequence,
            timestamp: row.timestamp,
            agent_id: row.agent_id,
            action: row.action,
            amount_wei: row.amount_wei.map(|d| d.to_string()),
            state_hash: row.state_hash,
        }
    }
}

#[async_trait]
impl GameStore for PgGameStore {
    async fn create_game(&self, game: &GameInstance) -> Result<GameInstance, AppError> {
        let row = sqlx::query_as::<_, GameRow>(
            r#"
            INSERT INTO games (game_id, room_id, game_type, game_version, status, current_state, sequence_number, created_at, started_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING game_id, room_id, game_type, game_version, status, current_state, sequence_number, created_at, started_at, completed_at
            "#,
        )
        .bind(game.game_id)
        .bind(game.room_id)
        .bind(&game.game_type)
        .bind(&game.game_version)
        .bind(&game.status)
        .bind(&game.current_state)
        .bind(game.sequence_number)
        .bind(game.created_at)
        .bind(game.started_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn get_game_by_id(&self, game_id: Uuid) -> Result<Option<GameInstance>, AppError> {
        let row = sqlx::query_as::<_, GameRow>(
            "SELECT game_id, room_id, game_type, game_version, status, current_state, sequence_number, created_at, started_at, completed_at FROM games WHERE game_id=$1",
        )
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn get_game_by_room_id(&self, room_id: Uuid) -> Result<Option<GameInstance>, AppError> {
        let row = sqlx::query_as::<_, GameRow>(
            "SELECT game_id, room_id, game_type, game_version, status, current_state, sequence_number, created_at, started_at, completed_at FROM games WHERE room_id=$1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn update_game_state(
        &self,
        game_id: Uuid,
        state: &serde_json::Value,
        sequence_number: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE games SET current_state=$2, sequence_number=$3 WHERE game_id=$1",
        )
        .bind(game_id)
        .bind(state)
        .bind(sequence_number)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn update_game_status(&self, game_id: Uuid, status: GameStatus) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE games SET status=$2, started_at = CASE WHEN $2='in_progress'::game_status THEN NOW() ELSE started_at END, completed_at = CASE WHEN $2 IN ('completed'::game_status,'cancelled'::game_status) THEN NOW() ELSE completed_at END WHERE game_id=$1",
        )
        .bind(game_id)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn list_games(
        &self,
        status: Option<GameStatus>,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<GameInstance>, AppError> {
        let rows = sqlx::query_as::<_, GameRow>(
            r#"
            SELECT game_id, room_id, game_type, game_version, status, current_state, sequence_number, created_at, started_at, completed_at
            FROM games
            WHERE ($1::game_status IS NULL OR status = $1)
              AND ($2::text IS NULL OR game_type = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(status)
        .bind(game_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create_player(&self, player: &GamePlayer) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO game_players (game_id, agent_id, wallet_address, seat_number, stack_wei, status, consecutive_timeouts) VALUES ($1, $2, $3, $4, $5::numeric, $6, $7)",
        )
        .bind(player.game_id)
        .bind(player.agent_id)
        .bind(&player.wallet_address)
        .bind(player.seat_number)
        .bind(&player.stack_wei)
        .bind(&player.status)
        .bind(player.consecutive_timeouts)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn get_players_by_game(&self, game_id: Uuid) -> Result<Vec<GamePlayer>, AppError> {
        let rows = sqlx::query_as::<_, GamePlayerRow>(
            "SELECT game_id, agent_id, wallet_address, seat_number, stack_wei, status, consecutive_timeouts FROM game_players WHERE game_id=$1 ORDER BY seat_number",
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_player(&self, player: &GamePlayer) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE game_players SET stack_wei=$3::numeric, status=$4, consecutive_timeouts=$5 WHERE game_id=$1 AND agent_id=$2",
        )
        .bind(player.game_id)
        .bind(player.agent_id)
        .bind(&player.stack_wei)
        .bind(&player.status)
        .bind(player.consecutive_timeouts)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn append_log_entry(&self, entry: &GameLogEntry) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO game_log (game_id, sequence, timestamp, agent_id, action, amount_wei, state_hash) VALUES ($1, $2, $3, $4, $5, $6::numeric, $7)",
        )
        .bind(entry.game_id)
        .bind(entry.sequence)
        .bind(entry.timestamp)
        .bind(entry.agent_id)
        .bind(&entry.action)
        .bind(&entry.amount_wei)
        .bind(&entry.state_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn get_log_by_game(&self, game_id: Uuid) -> Result<Vec<GameLogEntry>, AppError> {
        let rows = sqlx::query_as::<_, GameLogRow>(
            "SELECT game_id, sequence, timestamp, agent_id, action, amount_wei, state_hash FROM game_log WHERE game_id=$1 ORDER BY sequence",
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn save_result(&self, result: &GameResult) -> Result<(), AppError> {
        let winners = serde_json::to_value(&result.winners)
            .map_err(|e| AppError::Internal(e.into()))?;
        let losers = serde_json::to_value(&result.losers)
            .map_err(|e| AppError::Internal(e.into()))?;
        sqlx::query(
            "INSERT INTO game_results (game_id, winners, losers, rake_wei, rake_rate_bps, signed_result_hash) VALUES ($1, $2, $3, $4::numeric, $5, $6)",
        )
        .bind(result.game_id)
        .bind(winners)
        .bind(losers)
        .bind(&result.rake_wei)
        .bind(result.rake_rate_bps)
        .bind(&result.signed_result_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn get_result_by_game(&self, game_id: Uuid) -> Result<Option<GameResult>, AppError> {
        let row = sqlx::query(
            "SELECT game_id, winners, losers, rake_wei, rake_rate_bps, signed_result_hash FROM game_results WHERE game_id=$1",
        )
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        match row {
            None => Ok(None),
            Some(r) => {
                use sqlx::Row;
                let game_id: Uuid = r.get("game_id");
                let winners: serde_json::Value = r.get("winners");
                let losers: serde_json::Value = r.get("losers");
                let rake: sqlx::types::BigDecimal = r.get("rake_wei");
                let rake_rate_bps: i16 = r.get("rake_rate_bps");
                let signed_result_hash: String = r.get("signed_result_hash");

                let winners: Vec<WinnerEntry> = serde_json::from_value(winners)
                    .map_err(|e| AppError::Internal(e.into()))?;
                let losers: Vec<LoserEntry> = serde_json::from_value(losers)
                    .map_err(|e| AppError::Internal(e.into()))?;

                Ok(Some(GameResult {
                    game_id,
                    winners,
                    losers,
                    rake_wei: rake.to_string(),
                    rake_rate_bps,
                    signed_result_hash,
                }))
            }
        }
    }
}
