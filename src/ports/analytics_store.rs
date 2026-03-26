use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::analytics::{
    AgentActionEntry, AgentHandSummary, AgentMetrics, HeadToHeadRecord,
};
use crate::errors::AppError;

#[async_trait]
pub trait AnalyticsStore: Send + Sync + 'static {
    /// Fetch pre-computed metrics for an agent.
    async fn get_metrics(
        &self,
        agent_id: Uuid,
        game_type: &str,
    ) -> Result<Option<AgentMetrics>, AppError>;

    /// Upsert metrics (used by the computation worker / game completion hook).
    async fn upsert_metrics(&self, metrics: &AgentMetrics) -> Result<(), AppError>;

    /// Paginated raw action history for an agent.
    async fn list_actions(
        &self,
        agent_id: Uuid,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentActionEntry>, AppError>;

    /// Paginated hand summaries for an agent.
    async fn list_hands(
        &self,
        agent_id: Uuid,
        game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentHandSummary>, AppError>;

    /// Head-to-head record between two agents.
    async fn get_head_to_head(
        &self,
        agent_id: Uuid,
        opponent_id: Uuid,
        game_type: &str,
    ) -> Result<Option<HeadToHeadRecord>, AppError>;

    /// Upsert a head-to-head record.
    async fn upsert_head_to_head(&self, record: &HeadToHeadRecord) -> Result<(), AppError>;
}
