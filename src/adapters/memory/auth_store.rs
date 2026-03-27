use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::domain::auth::{Session, User};
use crate::errors::AppError;
use crate::ports::auth_store::AuthStore;

#[derive(Default)]
pub struct MemoryAuthStore {
    users: RwLock<HashMap<Uuid, User>>,
    sessions: RwLock<HashMap<Uuid, Session>>,
    public_keys: RwLock<HashMap<String, String>>,  // wallet → public_key_hex
}

impl MemoryAuthStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AuthStore for MemoryAuthStore {
    async fn upsert_user(&self, wallet: &str) -> Result<User, AppError> {
        let mut users = self.users.write().unwrap();
        if let Some(user) = users.values().find(|u| u.wallet == wallet) {
            return Ok(user.clone());
        }
        let now = Utc::now();
        let user = User {
            user_id: Uuid::new_v4(),
            wallet: wallet.to_string(),
            created_at: now,
            updated_at: now,
        };
        users.insert(user.user_id, user.clone());
        Ok(user)
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError> {
        Ok(self.users.read().unwrap().get(&user_id).cloned())
    }

    async fn get_user_by_wallet(&self, wallet: &str) -> Result<Option<User>, AppError> {
        Ok(self
            .users
            .read()
            .unwrap()
            .values()
            .find(|u| u.wallet == wallet)
            .cloned())
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Session, AppError> {
        let session = Session {
            session_id: Uuid::new_v4(),
            user_id,
            refresh_token: refresh_token.to_string(),
            expires_at,
            revoked: false,
            created_at: Utc::now(),
        };
        self.sessions
            .write()
            .unwrap()
            .insert(session.session_id, session.clone());
        Ok(session)
    }

    async fn get_session_by_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<Session>, AppError> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .find(|s| s.refresh_token == token)
            .cloned())
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), AppError> {
        if let Some(s) = self.sessions.write().unwrap().get_mut(&session_id) {
            s.revoked = true;
        }
        Ok(())
    }

    async fn set_public_key(&self, wallet: &str, public_key_hex: &str) -> Result<(), AppError> {
        self.public_keys.write().unwrap().insert(wallet.to_string(), public_key_hex.to_string());
        Ok(())
    }

    async fn get_public_key(&self, wallet: &str) -> Result<Option<String>, AppError> {
        Ok(self.public_keys.read().unwrap().get(wallet).cloned())
    }
}
