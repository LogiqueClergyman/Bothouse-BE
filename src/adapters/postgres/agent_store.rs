use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::agent::{Agent, AgentStats, AgentStatus};
use crate::errors::AppError;
use crate::ports::agent_store::AgentStore;

pub struct PgAgentStore {
    pool: PgPool,
}

impl PgAgentStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    agent_id: Uuid,
    user_id: Uuid,
    wallet_address: String,
    name: String,
    description: Option<String>,
    webhook_url: Option<String>,
    status: AgentStatus,
    api_key_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_seen_at: Option<DateTime<Utc>>,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        Self {
            agent_id: row.agent_id,
            user_id: row.user_id,
            wallet_address: row.wallet_address,
            name: row.name,
            description: row.description,
            webhook_url: row.webhook_url,
            status: row.status,
            api_key_hash: row.api_key_hash,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_seen_at: row.last_seen_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AgentStatsRow {
    agent_id: Uuid,
    game_type: String,
    games_played: i32,
    games_won: i32,
    total_wagered_atomic: sqlx::types::BigDecimal,
    total_won_atomic: sqlx::types::BigDecimal,
    total_lost_atomic: sqlx::types::BigDecimal,
    net_profit_atomic: sqlx::types::BigDecimal,
    win_rate: f64,
    updated_at: DateTime<Utc>,
}

impl From<AgentStatsRow> for AgentStats {
    fn from(row: AgentStatsRow) -> Self {
        use std::str::FromStr;
        Self {
            agent_id: row.agent_id,
            game_type: row.game_type,
            games_played: row.games_played,
            games_won: row.games_won,
            total_wagered_atomic: row.total_wagered_atomic.to_string(),
            total_won_atomic: row.total_won_atomic.to_string(),
            total_lost_atomic: row.total_lost_atomic.to_string(),
            net_profit_atomic: row.net_profit_atomic.to_string(),
            win_rate: row.win_rate,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl AgentStore for PgAgentStore {
    async fn create_agent(&self, agent: &Agent) -> Result<Agent, AppError> {
        let row = sqlx::query_as::<_, AgentRow>(
            r#"
            INSERT INTO agents (agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at
            "#,
        )
        .bind(agent.agent_id)
        .bind(agent.user_id)
        .bind(&agent.wallet_address)
        .bind(&agent.name)
        .bind(&agent.description)
        .bind(&agent.webhook_url)
        .bind(&agent.status)
        .bind(&agent.api_key_hash)
        .bind(agent.created_at)
        .bind(agent.updated_at)
        .bind(agent.last_seen_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn get_agent_by_id(&self, agent_id: Uuid) -> Result<Option<Agent>, AppError> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at FROM agents WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn get_agent_by_wallet(&self, wallet: &str) -> Result<Option<Agent>, AppError> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at FROM agents WHERE wallet_address = $1",
        )
        .bind(wallet)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn get_agent_by_api_key_hash(&self, hash: &str) -> Result<Option<Agent>, AppError> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at FROM agents WHERE api_key_hash = $1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.map(Into::into))
    }

    async fn list_agents_by_user(&self, user_id: Uuid) -> Result<Vec<Agent>, AppError> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at FROM agents WHERE user_id = $1 AND status != 'deleted' ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_agent(&self, agent: &Agent) -> Result<Agent, AppError> {
        let row = sqlx::query_as::<_, AgentRow>(
            r#"
            UPDATE agents SET name=$2, description=$3, webhook_url=$4, status=$5, api_key_hash=$6, updated_at=NOW(), last_seen_at=$7
            WHERE agent_id=$1
            RETURNING agent_id, user_id, wallet_address, name, description, webhook_url, status, api_key_hash, created_at, updated_at, last_seen_at
            "#,
        )
        .bind(agent.agent_id)
        .bind(&agent.name)
        .bind(&agent.description)
        .bind(&agent.webhook_url)
        .bind(&agent.status)
        .bind(&agent.api_key_hash)
        .bind(agent.last_seen_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(row.into())
    }

    async fn update_last_seen(&self, agent_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE agents SET last_seen_at=NOW() WHERE agent_id=$1")
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn get_stats(&self, agent_id: Uuid) -> Result<Vec<AgentStats>, AppError> {
        let rows = sqlx::query_as::<_, AgentStatsRow>(
            "SELECT agent_id, game_type, games_played, games_won, total_wagered_atomic, total_won_atomic, total_lost_atomic, net_profit_atomic, win_rate, updated_at FROM agent_stats WHERE agent_id=$1",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert_stats(&self, stats: &AgentStats) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO agent_stats (agent_id, game_type, games_played, games_won, total_wagered_atomic, total_won_atomic, total_lost_atomic, net_profit_atomic, win_rate, updated_at)
            VALUES ($1, $2, $3, $4, $5::numeric, $6::numeric, $7::numeric, $8::numeric, $9, NOW())
            ON CONFLICT (agent_id, game_type) DO UPDATE SET
              games_played = EXCLUDED.games_played,
              games_won = EXCLUDED.games_won,
              total_wagered_atomic = EXCLUDED.total_wagered_atomic,
              total_won_atomic = EXCLUDED.total_won_atomic,
              total_lost_atomic = EXCLUDED.total_lost_atomic,
              net_profit_atomic = EXCLUDED.net_profit_atomic,
              win_rate = EXCLUDED.win_rate,
              updated_at = NOW()
            "#,
        )
        .bind(stats.agent_id)
        .bind(&stats.game_type)
        .bind(stats.games_played)
        .bind(stats.games_won)
        .bind(&stats.total_wagered_atomic)
        .bind(&stats.total_won_atomic)
        .bind(&stats.total_lost_atomic)
        .bind(&stats.net_profit_atomic)
        .bind(stats.win_rate)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn get_leaderboard(
        &self,
        game_type: Option<&str>,
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(Agent, AgentStats)>, AppError> {
        let order_col = match sort_by {
            "win_rate" => "s.win_rate",
            "games_played" => "s.games_played",
            _ => "s.net_profit_atomic",
        };
        let sql = format!(
            r#"
            SELECT
              a.agent_id, a.user_id, a.wallet_address, a.name, a.description, a.webhook_url,
              a.status, a.api_key_hash, a.created_at, a.updated_at, a.last_seen_at,
              s.agent_id as s_agent_id, s.game_type, s.games_played, s.games_won,
              s.total_wagered_atomic, s.total_won_atomic, s.total_lost_atomic,
              s.net_profit_atomic, s.win_rate, s.updated_at as s_updated_at
            FROM agent_stats s
            JOIN agents a ON a.agent_id = s.agent_id
            WHERE ($1::text IS NULL OR s.game_type = $1)
              AND a.status = 'active'
            ORDER BY {} DESC
            LIMIT $2 OFFSET $3
            "#,
            order_col
        );

        let rows = sqlx::query(&sql)
            .bind(game_type)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        let mut result = Vec::new();
        for row in rows {
            use sqlx::Row;
            let agent = Agent {
                agent_id: row.get("agent_id"),
                user_id: row.get("user_id"),
                wallet_address: row.get("wallet_address"),
                name: row.get("name"),
                description: row.get("description"),
                webhook_url: row.get("webhook_url"),
                status: row.get("status"),
                api_key_hash: row.get("api_key_hash"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_seen_at: row.get("last_seen_at"),
            };
            let total_wagered: sqlx::types::BigDecimal = row.get("total_wagered_atomic");
            let total_won: sqlx::types::BigDecimal = row.get("total_won_atomic");
            let total_lost: sqlx::types::BigDecimal = row.get("total_lost_atomic");
            let net_profit: sqlx::types::BigDecimal = row.get("net_profit_atomic");
            let stats = AgentStats {
                agent_id: row.get("s_agent_id"),
                game_type: row.get("game_type"),
                games_played: row.get("games_played"),
                games_won: row.get("games_won"),
                total_wagered_atomic: total_wagered.to_string(),
                total_won_atomic: total_won.to_string(),
                total_lost_atomic: total_lost.to_string(),
                net_profit_atomic: net_profit.to_string(),
                win_rate: row.get("win_rate"),
                updated_at: row.get("s_updated_at"),
            };
            result.push((agent, stats));
        }
        Ok(result)
    }
}
