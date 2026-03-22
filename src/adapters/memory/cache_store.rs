use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::errors::AppError;
use crate::ports::cache_store::CacheStore;

#[derive(Default)]
pub struct MemoryCacheStore {
    data: RwLock<HashMap<String, String>>,
}

impl MemoryCacheStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CacheStore for MemoryCacheStore {
    async fn set_nonce(&self, wallet: &str, nonce: &str) -> Result<(), AppError> {
        self.data.write().unwrap().insert(format!("nonce:{}", wallet), nonce.to_string());
        Ok(())
    }

    async fn get_nonce(&self, wallet: &str) -> Result<Option<String>, AppError> {
        Ok(self.data.read().unwrap().get(&format!("nonce:{}", wallet)).cloned())
    }

    async fn delete_nonce(&self, wallet: &str) -> Result<(), AppError> {
        self.data.write().unwrap().remove(&format!("nonce:{}", wallet));
        Ok(())
    }

    async fn set_session(&self, session_id: &str, user_id: &str, _ttl_secs: u64) -> Result<(), AppError> {
        self.data.write().unwrap().insert(format!("session:{}", session_id), user_id.to_string());
        Ok(())
    }

    async fn get_session_user(&self, session_id: &str) -> Result<Option<String>, AppError> {
        Ok(self.data.read().unwrap().get(&format!("session:{}", session_id)).cloned())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), AppError> {
        self.data.write().unwrap().remove(&format!("session:{}", session_id));
        Ok(())
    }

    async fn set_agent_key(&self, key_prefix: &str, agent_id: &str) -> Result<(), AppError> {
        self.data.write().unwrap().insert(format!("agent_key:{}", key_prefix), agent_id.to_string());
        Ok(())
    }

    async fn get_agent_by_key(&self, key_prefix: &str) -> Result<Option<String>, AppError> {
        Ok(self.data.read().unwrap().get(&format!("agent_key:{}", key_prefix)).cloned())
    }

    async fn set_current_turn(&self, game_id: &str, agent_id: &str, _ttl_ms: u64) -> Result<(), AppError> {
        self.data.write().unwrap().insert(format!("turn:{}", game_id), agent_id.to_string());
        Ok(())
    }

    async fn get_current_turn(&self, game_id: &str) -> Result<Option<String>, AppError> {
        Ok(self.data.read().unwrap().get(&format!("turn:{}", game_id)).cloned())
    }

    async fn delete_current_turn(&self, game_id: &str) -> Result<(), AppError> {
        self.data.write().unwrap().remove(&format!("turn:{}", game_id));
        Ok(())
    }
}
