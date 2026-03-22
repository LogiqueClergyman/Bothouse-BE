use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::auth::{Session, User};
use crate::errors::AppError;
use crate::ports::auth_store::AuthStore;

pub struct PgAuthStore {
    pool: PgPool,
}

impl PgAuthStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    user_id: Uuid,
    wallet: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            user_id: row.user_id,
            wallet: row.wallet,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: Uuid,
    user_id: Uuid,
    refresh_token: String,
    expires_at: DateTime<Utc>,
    revoked: bool,
    created_at: DateTime<Utc>,
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Self {
            session_id: row.session_id,
            user_id: row.user_id,
            refresh_token: row.refresh_token,
            expires_at: row.expires_at,
            revoked: row.revoked,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl AuthStore for PgAuthStore {
    async fn upsert_user(&self, wallet: &str) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (wallet)
            VALUES ($1)
            ON CONFLICT (wallet) DO UPDATE
              SET updated_at = NOW()
            RETURNING user_id, wallet, created_at, updated_at
            "#,
        )
        .bind(wallet)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT user_id, wallet, created_at, updated_at FROM users WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn get_user_by_wallet(&self, wallet: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT user_id, wallet, created_at, updated_at FROM users WHERE wallet = $1",
        )
        .bind(wallet)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Session, AppError> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            INSERT INTO sessions (user_id, refresh_token, expires_at)
            VALUES ($1, $2, $3)
            RETURNING session_id, user_id, refresh_token, expires_at, revoked, created_at
            "#,
        )
        .bind(user_id)
        .bind(refresh_token)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn get_session_by_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<Session>, AppError> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT session_id, user_id, refresh_token, expires_at, revoked, created_at
            FROM sessions WHERE refresh_token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE sessions SET revoked = TRUE WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }
}
