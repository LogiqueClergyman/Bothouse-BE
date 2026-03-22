use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::game::{
    GameInstance, GameLogEntry, GamePlayer, GameResult, GameStatus,
};
use crate::errors::AppError;

#[async_trait]
pub trait GameStore: Send + Sync + 'static {
    async fn create_game(&self, game: &GameInstance) -> Result<GameInstance, AppError>;
    async fn get_game_by_id(&self, game_id: Uuid) -> Result<Option<GameInstance>, AppError>;
    async fn get_game_by_room_id(&self, room_id: Uuid) -> Result<Option<GameInstance>, AppError>;
    async fn update_game_state(
        &self,
        game_id: Uuid,
        state: &serde_json::Value,
        sequence_number: i64,
    ) -> Result<(), AppError>;
    async fn update_game_status(&self, game_id: Uuid, status: GameStatus) -> Result<(), AppError>;
    async fn list_games(
        &self,
        status: Option<GameStatus>,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<GameInstance>, AppError>;
    async fn create_player(&self, player: &GamePlayer) -> Result<(), AppError>;
    async fn get_players_by_game(&self, game_id: Uuid) -> Result<Vec<GamePlayer>, AppError>;
    async fn update_player(&self, player: &GamePlayer) -> Result<(), AppError>;
    async fn append_log_entry(&self, entry: &GameLogEntry) -> Result<(), AppError>;
    async fn get_log_by_game(&self, game_id: Uuid) -> Result<Vec<GameLogEntry>, AppError>;
    async fn save_result(&self, result: &GameResult) -> Result<(), AppError>;
    async fn get_result_by_game(&self, game_id: Uuid) -> Result<Option<GameResult>, AppError>;
}
