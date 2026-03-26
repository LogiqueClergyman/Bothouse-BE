use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::domain::analytics::{
    AgentActionEntry, AgentHandSummary, AgentMetrics, HeadToHeadRecord,
};
use crate::errors::AppError;
use crate::ports::analytics_store::AnalyticsStore;

#[derive(Default)]
pub struct MemoryAnalyticsStore {
    metrics: RwLock<HashMap<(Uuid, String), AgentMetrics>>,
    actions: RwLock<HashMap<Uuid, Vec<AgentActionEntry>>>,
    hands: RwLock<HashMap<Uuid, Vec<AgentHandSummary>>>,
    h2h: RwLock<HashMap<(Uuid, Uuid, String), HeadToHeadRecord>>,
}

impl MemoryAnalyticsStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AnalyticsStore for MemoryAnalyticsStore {
    async fn get_metrics(
        &self,
        agent_id: Uuid,
        game_type: &str,
    ) -> Result<Option<AgentMetrics>, AppError> {
        Ok(self
            .metrics
            .read()
            .unwrap()
            .get(&(agent_id, game_type.to_string()))
            .cloned())
    }

    async fn upsert_metrics(&self, metrics: &AgentMetrics) -> Result<(), AppError> {
        self.metrics
            .write()
            .unwrap()
            .insert((metrics.agent_id, metrics.game_type.clone()), metrics.clone());
        Ok(())
    }

    async fn list_actions(
        &self,
        agent_id: Uuid,
        _game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentActionEntry>, AppError> {
        let store = self.actions.read().unwrap();
        let all = store.get(&agent_id).cloned().unwrap_or_default();
        Ok(all
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn list_hands(
        &self,
        agent_id: Uuid,
        _game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentHandSummary>, AppError> {
        let store = self.hands.read().unwrap();
        let all = store.get(&agent_id).cloned().unwrap_or_default();
        Ok(all
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn get_head_to_head(
        &self,
        agent_id: Uuid,
        opponent_id: Uuid,
        game_type: &str,
    ) -> Result<Option<HeadToHeadRecord>, AppError> {
        Ok(self
            .h2h
            .read()
            .unwrap()
            .get(&(agent_id, opponent_id, game_type.to_string()))
            .cloned())
    }

    async fn upsert_head_to_head(&self, record: &HeadToHeadRecord) -> Result<(), AppError> {
        self.h2h.write().unwrap().insert(
            (record.agent_id, record.opponent_id, record.game_type.clone()),
            record.clone(),
        );
        Ok(())
    }
}
