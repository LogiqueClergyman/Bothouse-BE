use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::game::WinnerEntry;
use crate::errors::AppError;

#[async_trait]
pub trait SettlementPort: Send + Sync + 'static {
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
