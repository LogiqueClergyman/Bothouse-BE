use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::lobby::{Room, RoomStatus, Seat};
use crate::errors::AppError;
use crate::ports::lobby_store::LobbyStore;

pub struct PgLobbyStore {
    pool: PgPool,
}

impl PgLobbyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RoomRow {
    room_id: Uuid,
    game_type: String,
    game_version: String,
    status: RoomStatus,
    buy_in_wei: sqlx::types::BigDecimal,
    max_players: i16,
    min_players: i16,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<RoomRow> for Room {
    fn from(row: RoomRow) -> Self {
        Self {
            room_id: row.room_id,
            game_type: row.game_type,
            game_version: row.game_version,
            status: row.status,
            buy_in_wei: row.buy_in_wei.to_string(),
            max_players: row.max_players,
            min_players: row.min_players,
            created_at: row.created_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SeatRow {
    seat_id: Uuid,
    room_id: Uuid,
    agent_id: Uuid,
    wallet_address: String,
    seat_number: i16,
    joined_at: DateTime<Utc>,
    escrow_tx_hash: Option<String>,
    escrow_verified: bool,
}

impl From<SeatRow> for Seat {
    fn from(row: SeatRow) -> Self {
        Self {
            seat_id: row.seat_id,
            room_id: row.room_id,
            agent_id: row.agent_id,
            wallet_address: row.wallet_address,
            seat_number: row.seat_number,
            joined_at: row.joined_at,
            escrow_tx_hash: row.escrow_tx_hash,
            escrow_verified: row.escrow_verified,
        }
    }
}

#[async_trait]
impl LobbyStore for PgLobbyStore {
    async fn create_room(&self, room: &Room) -> Result<Room, AppError> {
        let row = sqlx::query_as::<_, RoomRow>(
            r#"
            INSERT INTO rooms (room_id, game_type, game_version, status, buy_in_wei, max_players, min_players, created_at)
            VALUES ($1, $2, $3, $4, $5::numeric, $6, $7, $8)
            RETURNING room_id, game_type, game_version, status, buy_in_wei, max_players, min_players, created_at, started_at, completed_at
            "#,
        )
        .bind(room.room_id)
        .bind(&room.game_type)
        .bind(&room.game_version)
        .bind(&room.status)
        .bind(&room.buy_in_wei)
        .bind(room.max_players)
        .bind(room.min_players)
        .bind(room.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, AppError> {
        let row = sqlx::query_as::<_, RoomRow>(
            "SELECT room_id, game_type, game_version, status, buy_in_wei, max_players, min_players, created_at, started_at, completed_at FROM rooms WHERE room_id=$1",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn list_rooms(
        &self,
        status: Option<RoomStatus>,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Room>, AppError> {
        let rows = sqlx::query_as::<_, RoomRow>(
            r#"
            SELECT room_id, game_type, game_version, status, buy_in_wei, max_players, min_players, created_at, started_at, completed_at
            FROM rooms
            WHERE ($1::room_status IS NULL OR status = $1)
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

    async fn update_room_status(&self, room_id: Uuid, status: RoomStatus) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE rooms SET status=$2, started_at = CASE WHEN $2='in_progress'::room_status THEN NOW() ELSE started_at END, completed_at = CASE WHEN $2 IN ('completed'::room_status,'cancelled'::room_status) THEN NOW() ELSE completed_at END WHERE room_id=$1",
        )
        .bind(room_id)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn create_seat(&self, seat: &Seat) -> Result<Seat, AppError> {
        let row = sqlx::query_as::<_, SeatRow>(
            r#"
            INSERT INTO seats (seat_id, room_id, agent_id, wallet_address, seat_number, joined_at, escrow_tx_hash, escrow_verified)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING seat_id, room_id, agent_id, wallet_address, seat_number, joined_at, escrow_tx_hash, escrow_verified
            "#,
        )
        .bind(seat.seat_id)
        .bind(seat.room_id)
        .bind(seat.agent_id)
        .bind(&seat.wallet_address)
        .bind(seat.seat_number)
        .bind(seat.joined_at)
        .bind(&seat.escrow_tx_hash)
        .bind(seat.escrow_verified)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn get_seats_by_room(&self, room_id: Uuid) -> Result<Vec<Seat>, AppError> {
        let rows = sqlx::query_as::<_, SeatRow>(
            "SELECT seat_id, room_id, agent_id, wallet_address, seat_number, joined_at, escrow_tx_hash, escrow_verified FROM seats WHERE room_id=$1 ORDER BY seat_number",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_seat_by_agent_and_room(
        &self,
        agent_id: Uuid,
        room_id: Uuid,
    ) -> Result<Option<Seat>, AppError> {
        let row = sqlx::query_as::<_, SeatRow>(
            "SELECT seat_id, room_id, agent_id, wallet_address, seat_number, joined_at, escrow_tx_hash, escrow_verified FROM seats WHERE agent_id=$1 AND room_id=$2",
        )
        .bind(agent_id)
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn update_seat_escrow(
        &self,
        seat_id: Uuid,
        tx_hash: &str,
        verified: bool,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE seats SET escrow_tx_hash=$2, escrow_verified=$3 WHERE seat_id=$1",
        )
        .bind(seat_id)
        .bind(tx_hash)
        .bind(verified)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn delete_seat(&self, seat_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM seats WHERE seat_id=$1")
            .bind(seat_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }
}
