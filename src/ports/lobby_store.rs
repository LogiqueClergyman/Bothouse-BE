use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::lobby::{Room, RoomStatus, Seat};
use crate::errors::AppError;

#[async_trait]
pub trait LobbyStore: Send + Sync + 'static {
    async fn create_room(&self, room: &Room) -> Result<Room, AppError>;
    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, AppError>;
    async fn list_rooms(
        &self,
        status: Option<RoomStatus>,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Room>, AppError>;
    async fn update_room_status(&self, room_id: Uuid, status: RoomStatus) -> Result<(), AppError>;
    async fn create_seat(&self, seat: &Seat) -> Result<Seat, AppError>;
    async fn get_seats_by_room(&self, room_id: Uuid) -> Result<Vec<Seat>, AppError>;
    async fn get_seat_by_agent_and_room(
        &self,
        agent_id: Uuid,
        room_id: Uuid,
    ) -> Result<Option<Seat>, AppError>;
    async fn update_seat_escrow(
        &self,
        seat_id: Uuid,
        tx_hash: &str,
        verified: bool,
    ) -> Result<(), AppError>;
    async fn delete_seat(&self, seat_id: Uuid) -> Result<(), AppError>;
}
