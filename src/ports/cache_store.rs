use async_trait::async_trait;

use crate::errors::AppError;

#[async_trait]
pub trait CacheStore: Send + Sync + 'static {
    async fn set_nonce(&self, wallet: &str, nonce: &str) -> Result<(), AppError>;
    async fn get_nonce(&self, wallet: &str) -> Result<Option<String>, AppError>;
    async fn delete_nonce(&self, wallet: &str) -> Result<(), AppError>;

    async fn set_session(
        &self,
        session_id: &str,
        user_id: &str,
        ttl_secs: u64,
    ) -> Result<(), AppError>;
    async fn get_session_user(&self, session_id: &str) -> Result<Option<String>, AppError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), AppError>;

    async fn set_agent_key(&self, key_prefix: &str, agent_id: &str) -> Result<(), AppError>;
    async fn get_agent_by_key(&self, key_prefix: &str) -> Result<Option<String>, AppError>;

    async fn set_current_turn(
        &self,
        game_id: &str,
        agent_id: &str,
        ttl_ms: u64,
    ) -> Result<(), AppError>;
    async fn get_current_turn(&self, game_id: &str) -> Result<Option<String>, AppError>;
    async fn delete_current_turn(&self, game_id: &str) -> Result<(), AppError>;
}
