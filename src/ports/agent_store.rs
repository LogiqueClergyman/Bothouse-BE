use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::agent::{Agent, AgentStats};
use crate::errors::AppError;

#[async_trait]
pub trait AgentStore: Send + Sync + 'static {
    async fn create_agent(&self, agent: &Agent) -> Result<Agent, AppError>;
    async fn get_agent_by_id(&self, agent_id: Uuid) -> Result<Option<Agent>, AppError>;
    async fn get_agent_by_wallet(&self, wallet: &str) -> Result<Option<Agent>, AppError>;
    async fn get_agent_by_api_key_hash(&self, hash: &str) -> Result<Option<Agent>, AppError>;
    async fn list_agents_by_user(&self, user_id: Uuid) -> Result<Vec<Agent>, AppError>;
    async fn update_agent(&self, agent: &Agent) -> Result<Agent, AppError>;
    async fn update_last_seen(&self, agent_id: Uuid) -> Result<(), AppError>;
    async fn get_stats(&self, agent_id: Uuid) -> Result<Vec<AgentStats>, AppError>;
    async fn upsert_stats(&self, stats: &AgentStats) -> Result<(), AppError>;
    async fn get_leaderboard(
        &self,
        game_type: Option<&str>,
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(Agent, AgentStats)>, AppError>;
}
