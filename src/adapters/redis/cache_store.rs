use async_trait::async_trait;
use redis::AsyncCommands;

use crate::errors::AppError;
use crate::ports::cache_store::CacheStore;

pub struct RedisCache {
    conn: redis::aio::ConnectionManager,
}

impl RedisCache {
    pub fn new(conn: redis::aio::ConnectionManager) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl CacheStore for RedisCache {
    async fn set_nonce(&self, wallet: &str, nonce: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        conn.set_ex(format!("nonce:{}", wallet), nonce, 300)
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn get_nonce(&self, wallet: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        conn.get(format!("nonce:{}", wallet))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn delete_nonce(&self, wallet: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        conn.del(format!("nonce:{}", wallet))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn set_session(
        &self,
        session_id: &str,
        user_id: &str,
        ttl_secs: u64,
    ) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        conn.set_ex(format!("session:{}", session_id), user_id, ttl_secs)
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn get_session_user(&self, session_id: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        conn.get(format!("session:{}", session_id))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        conn.del(format!("session:{}", session_id))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn set_agent_key(&self, key_prefix: &str, agent_id: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        conn.set(format!("agent_key:{}", key_prefix), agent_id)
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn get_agent_by_key(&self, key_prefix: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        conn.get(format!("agent_key:{}", key_prefix))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn set_current_turn(
        &self,
        game_id: &str,
        agent_id: &str,
        ttl_ms: u64,
    ) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        // Use 2x TTL so the key survives until the turn manager wakes up to handle timeout
        let ttl_secs = ((ttl_ms / 1000) * 2).max(2);
        conn.set_ex(format!("turn:{}", game_id), agent_id, ttl_secs)
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn get_current_turn(&self, game_id: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.conn.clone();
        conn.get(format!("turn:{}", game_id))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }

    async fn delete_current_turn(&self, game_id: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        conn.del(format!("turn:{}", game_id))
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }
}
