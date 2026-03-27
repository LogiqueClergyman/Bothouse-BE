use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::auth::{Session, User};
use crate::errors::AppError;

#[async_trait]
pub trait AuthStore: Send + Sync + 'static {
    async fn upsert_user(&self, wallet: &str) -> Result<User, AppError>;
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError>;
    async fn get_user_by_wallet(&self, wallet: &str) -> Result<Option<User>, AppError>;
    async fn create_session(
        &self,
        user_id: Uuid,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Session, AppError>;
    async fn get_session_by_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<Session>, AppError>;
    async fn revoke_session(&self, session_id: Uuid) -> Result<(), AppError>;
    /// Store a public key for a wallet (used by OneChain Ed25519 auth).
    async fn set_public_key(&self, wallet: &str, public_key_hex: &str) -> Result<(), AppError>;
    /// Retrieve the stored public key for a wallet (OneChain Ed25519 verification).
    async fn get_public_key(&self, wallet: &str) -> Result<Option<String>, AppError>;
}
