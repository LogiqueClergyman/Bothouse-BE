use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::game::WinnerEntry;
use crate::errors::AppError;

#[async_trait]
pub trait SettlementPort: Send + Sync + 'static {
    /// Create the on-chain escrow game for this `game_id` (room_id).
    /// Returns a transaction hash/digest.
    async fn create_game(&self, game_id: Uuid, buy_in_atomic: &str) -> Result<String, AppError>;

    /// Mark the game as started on-chain (EVM) if applicable.
    /// Returns a transaction hash/digest (or empty string if not applicable on the chain).
    async fn start_game(&self, game_id: Uuid) -> Result<String, AppError>;

    async fn settle(
        &self,
        game_id: Uuid,
        winners: &[WinnerEntry],
        rake_atomic: &str,
        result_hash: &str,
    ) -> Result<String, AppError>;

    async fn check_confirmation(&self, tx_hash: &str) -> Result<Option<i64>, AppError>;

    async fn check_escrow_deposit(
        &self,
        game_id: Uuid,
        wallet: &str,
        buy_in_atomic: &str,
    ) -> Result<bool, AppError>;
}
