use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::domain::agent::{Agent, AgentStats};
use crate::errors::AppError;
use crate::ports::agent_store::AgentStore;

#[derive(Default)]
pub struct MemoryAgentStore {
    agents: RwLock<HashMap<Uuid, Agent>>,
    stats: RwLock<HashMap<(Uuid, String), AgentStats>>,
}

impl MemoryAgentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AgentStore for MemoryAgentStore {
    async fn create_agent(&self, agent: &Agent) -> Result<Agent, AppError> {
        self.agents
            .write()
            .unwrap()
            .insert(agent.agent_id, agent.clone());
        Ok(agent.clone())
    }

    async fn get_agent_by_id(&self, agent_id: Uuid) -> Result<Option<Agent>, AppError> {
        Ok(self.agents.read().unwrap().get(&agent_id).cloned())
    }

    async fn get_agent_by_wallet(&self, wallet: &str) -> Result<Option<Agent>, AppError> {
        Ok(self
            .agents
            .read()
            .unwrap()
            .values()
            .find(|a| a.wallet_address.to_lowercase() == wallet.to_lowercase())
            .cloned())
    }

    async fn get_agent_by_api_key_hash(&self, hash: &str) -> Result<Option<Agent>, AppError> {
        Ok(self
            .agents
            .read()
            .unwrap()
            .values()
            .find(|a| a.api_key_hash == hash)
            .cloned())
    }

    async fn list_agents_by_user(&self, user_id: Uuid) -> Result<Vec<Agent>, AppError> {
        Ok(self
            .agents
            .read()
            .unwrap()
            .values()
            .filter(|a| a.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn update_agent(&self, agent: &Agent) -> Result<Agent, AppError> {
        self.agents
            .write()
            .unwrap()
            .insert(agent.agent_id, agent.clone());
        Ok(agent.clone())
    }

    async fn update_last_seen(&self, agent_id: Uuid) -> Result<(), AppError> {
        if let Some(a) = self.agents.write().unwrap().get_mut(&agent_id) {
            a.last_seen_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn get_stats(&self, agent_id: Uuid) -> Result<Vec<AgentStats>, AppError> {
        Ok(self
            .stats
            .read()
            .unwrap()
            .iter()
            .filter(|((id, _), _)| *id == agent_id)
            .map(|(_, s)| s.clone())
            .collect())
    }

    async fn upsert_stats(&self, stats: &AgentStats) -> Result<(), AppError> {
        self.stats
            .write()
            .unwrap()
            .insert((stats.agent_id, stats.game_type.clone()), stats.clone());
        Ok(())
    }

    async fn get_leaderboard(
        &self,
        game_type: Option<&str>,
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(Agent, AgentStats)>, AppError> {
        let agents = self.agents.read().unwrap();
        let stats = self.stats.read().unwrap();

        let mut pairs: Vec<(Agent, AgentStats)> = stats
            .iter()
            .filter(|((_, gt), _)| game_type.map_or(true, |g| gt == g))
            .filter_map(|((agent_id, _), s)| {
                agents.get(agent_id).map(|a| (a.clone(), s.clone()))
            })
            .collect();

        pairs.sort_by(|(_, a), (_, b)| match sort_by {
            "win_rate" => b.win_rate.partial_cmp(&a.win_rate).unwrap_or(std::cmp::Ordering::Equal),
            "games_played" => b.games_played.cmp(&a.games_played),
            _ => {
                let a_val: i128 = a.net_profit_atomic.parse().unwrap_or(0);
                let b_val: i128 = b.net_profit_atomic.parse().unwrap_or(0);
                b_val.cmp(&a_val)
            }
        });

        Ok(pairs
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }
}
