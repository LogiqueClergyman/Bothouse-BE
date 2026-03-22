use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::domain::lobby::{Room, RoomStatus, Seat};
use crate::errors::AppError;
use crate::ports::lobby_store::LobbyStore;

#[derive(Default)]
pub struct MemoryLobbyStore {
    rooms: RwLock<HashMap<Uuid, Room>>,
    seats: RwLock<HashMap<Uuid, Seat>>,
}

impl MemoryLobbyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl LobbyStore for MemoryLobbyStore {
    async fn create_room(&self, room: &Room) -> Result<Room, AppError> {
        self.rooms
            .write()
            .unwrap()
            .insert(room.room_id, room.clone());
        Ok(room.clone())
    }

    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, AppError> {
        Ok(self.rooms.read().unwrap().get(&room_id).cloned())
    }

    async fn list_rooms(
        &self,
        status: Option<RoomStatus>,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Room>, AppError> {
        let rooms = self.rooms.read().unwrap();
        let mut result: Vec<Room> = rooms
            .values()
            .filter(|r| status.as_ref().map_or(true, |s| &r.status == s))
            .filter(|r| game_type.map_or(true, |g| r.game_type == g))
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn update_room_status(&self, room_id: Uuid, status: RoomStatus) -> Result<(), AppError> {
        if let Some(r) = self.rooms.write().unwrap().get_mut(&room_id) {
            r.status = status;
        }
        Ok(())
    }

    async fn create_seat(&self, seat: &Seat) -> Result<Seat, AppError> {
        self.seats
            .write()
            .unwrap()
            .insert(seat.seat_id, seat.clone());
        Ok(seat.clone())
    }

    async fn get_seats_by_room(&self, room_id: Uuid) -> Result<Vec<Seat>, AppError> {
        let mut seats: Vec<Seat> = self
            .seats
            .read()
            .unwrap()
            .values()
            .filter(|s| s.room_id == room_id)
            .cloned()
            .collect();
        seats.sort_by_key(|s| s.seat_number);
        Ok(seats)
    }

    async fn get_seat_by_agent_and_room(
        &self,
        agent_id: Uuid,
        room_id: Uuid,
    ) -> Result<Option<Seat>, AppError> {
        Ok(self
            .seats
            .read()
            .unwrap()
            .values()
            .find(|s| s.agent_id == agent_id && s.room_id == room_id)
            .cloned())
    }

    async fn update_seat_escrow(
        &self,
        seat_id: Uuid,
        tx_hash: &str,
        verified: bool,
    ) -> Result<(), AppError> {
        if let Some(s) = self.seats.write().unwrap().get_mut(&seat_id) {
            s.escrow_tx_hash = Some(tx_hash.to_string());
            s.escrow_verified = verified;
        }
        Ok(())
    }

    async fn delete_seat(&self, seat_id: Uuid) -> Result<(), AppError> {
        self.seats.write().unwrap().remove(&seat_id);
        Ok(())
    }
}
