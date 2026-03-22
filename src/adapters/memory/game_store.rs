use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::domain::game::{
    GameInstance, GameLogEntry, GamePlayer, GameResult, GameStatus,
};
use crate::errors::AppError;
use crate::ports::game_store::GameStore;

#[derive(Default)]
pub struct MemoryGameStore {
    games: RwLock<HashMap<Uuid, GameInstance>>,
    players: RwLock<HashMap<Uuid, Vec<GamePlayer>>>,
    logs: RwLock<HashMap<Uuid, Vec<GameLogEntry>>>,
    results: RwLock<HashMap<Uuid, GameResult>>,
}

impl MemoryGameStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl GameStore for MemoryGameStore {
    async fn create_game(&self, game: &GameInstance) -> Result<GameInstance, AppError> {
        self.games
            .write()
            .unwrap()
            .insert(game.game_id, game.clone());
        Ok(game.clone())
    }

    async fn get_game_by_id(&self, game_id: Uuid) -> Result<Option<GameInstance>, AppError> {
        Ok(self.games.read().unwrap().get(&game_id).cloned())
    }

    async fn get_game_by_room_id(&self, room_id: Uuid) -> Result<Option<GameInstance>, AppError> {
        Ok(self
            .games
            .read()
            .unwrap()
            .values()
            .find(|g| g.room_id == room_id)
            .cloned())
    }

    async fn update_game_state(
        &self,
        game_id: Uuid,
        state: &serde_json::Value,
        sequence_number: i64,
    ) -> Result<(), AppError> {
        if let Some(g) = self.games.write().unwrap().get_mut(&game_id) {
            g.current_state = state.clone();
            g.sequence_number = sequence_number;
        }
        Ok(())
    }

    async fn update_game_status(&self, game_id: Uuid, status: GameStatus) -> Result<(), AppError> {
        if let Some(g) = self.games.write().unwrap().get_mut(&game_id) {
            g.status = status;
        }
        Ok(())
    }

    async fn list_games(
        &self,
        status: Option<GameStatus>,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<GameInstance>, AppError> {
        let games = self.games.read().unwrap();
        let mut result: Vec<GameInstance> = games
            .values()
            .filter(|g| status.as_ref().map_or(true, |s| &g.status == s))
            .filter(|g| game_type.map_or(true, |t| g.game_type == t))
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn create_player(&self, player: &GamePlayer) -> Result<(), AppError> {
        self.players
            .write()
            .unwrap()
            .entry(player.game_id)
            .or_default()
            .push(player.clone());
        Ok(())
    }

    async fn get_players_by_game(&self, game_id: Uuid) -> Result<Vec<GamePlayer>, AppError> {
        Ok(self
            .players
            .read()
            .unwrap()
            .get(&game_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn update_player(&self, player: &GamePlayer) -> Result<(), AppError> {
        if let Some(players) = self.players.write().unwrap().get_mut(&player.game_id) {
            if let Some(p) = players.iter_mut().find(|p| p.agent_id == player.agent_id) {
                *p = player.clone();
            }
        }
        Ok(())
    }

    async fn append_log_entry(&self, entry: &GameLogEntry) -> Result<(), AppError> {
        self.logs
            .write()
            .unwrap()
            .entry(entry.game_id)
            .or_default()
            .push(entry.clone());
        Ok(())
    }

    async fn get_log_by_game(&self, game_id: Uuid) -> Result<Vec<GameLogEntry>, AppError> {
        let mut entries = self
            .logs
            .read()
            .unwrap()
            .get(&game_id)
            .cloned()
            .unwrap_or_default();
        entries.sort_by_key(|e| e.sequence);
        Ok(entries)
    }

    async fn save_result(&self, result: &GameResult) -> Result<(), AppError> {
        self.results
            .write()
            .unwrap()
            .insert(result.game_id, result.clone());
        Ok(())
    }

    async fn get_result_by_game(&self, game_id: Uuid) -> Result<Option<GameResult>, AppError> {
        Ok(self.results.read().unwrap().get(&game_id).cloned())
    }
}
