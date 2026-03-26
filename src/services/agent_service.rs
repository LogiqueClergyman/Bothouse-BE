use chrono::Utc;
use uuid::Uuid;

use crate::domain::agent::{Agent, AgentStats, AgentStatus};
use crate::errors::AppError;
use crate::state::AppState;

fn is_valid_evm_address(addr: &str) -> bool {
    addr.len() == 42
        && addr.starts_with("0x")
        && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub struct RegisterAgentRequest {
    pub name: String,
    pub wallet_address: String,
    pub description: Option<String>,
    pub webhook_url: Option<String>,
}

pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub webhook_url: Option<String>,
    pub status: Option<AgentStatus>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AgentView {
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub wallet_address: String,
    pub name: String,
    pub description: Option<String>,
    pub webhook_url: Option<String>,
    pub status: AgentStatus,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub last_seen_at: Option<chrono::DateTime<Utc>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub agent: AgentPublic,
    pub stats: AgentStats,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AgentPublic {
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub name: String,
    pub description: Option<String>,
    pub status: AgentStatus,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<Agent> for AgentPublic {
    fn from(a: Agent) -> Self {
        AgentPublic {
            agent_id: a.agent_id,
            wallet_address: a.wallet_address,
            name: a.name,
            description: a.description,
            status: a.status,
            created_at: a.created_at,
        }
    }
}

pub async fn register_agent(
    user_id: Uuid,
    req: RegisterAgentRequest,
    state: &AppState,
) -> Result<(Agent, String), AppError> {
    if req.name.is_empty() || req.name.len() > 32 {
        return Err(AppError::BadRequest(
            "Name must be 1-32 characters".to_string(),
        ));
    }
    if let Some(ref desc) = req.description {
        if desc.len() > 256 {
            return Err(AppError::BadRequest(
                "Description max 256 characters".to_string(),
            ));
        }
    }
    if let Some(ref url) = req.webhook_url {
        if url.len() > 512 || !is_valid_url(url) {
            return Err(AppError::BadRequest("Invalid webhook URL".to_string()));
        }
    }
    if !is_valid_evm_address(&req.wallet_address) {
        return Err(AppError::BadRequest(
            "Invalid wallet address".to_string(),
        ));
    }

    let existing = state
        .agent_store
        .get_agent_by_wallet(&req.wallet_address.to_lowercase())
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "Wallet already registered to an agent".to_string(),
        ));
    }

    let raw_key = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        format!("bth_{}", hex::encode(bytes))
    };

    let key_hash =
        bcrypt::hash(&raw_key, state.config.bcrypt_cost).map_err(|e| AppError::Internal(e.into()))?;

    let now = Utc::now();
    let agent = Agent {
        agent_id: Uuid::new_v4(),
        user_id,
        wallet_address: req.wallet_address.to_lowercase(),
        name: req.name,
        description: req.description,
        webhook_url: req.webhook_url,
        status: AgentStatus::Active,
        api_key_hash: key_hash,
        created_at: now,
        updated_at: now,
        last_seen_at: None,
    };

    let agent = state.agent_store.create_agent(&agent).await?;

    // Cache api_key_prefix → agent_id
    let key_prefix = &raw_key[4..20]; // first 16 chars after "bth_"
    state
        .cache
        .set_agent_key(key_prefix, &agent.agent_id.to_string())
        .await?;

    Ok((agent, raw_key))
}

pub async fn get_agent(
    agent_id: Uuid,
    requesting_user_id: Option<Uuid>,
    state: &AppState,
) -> Result<AgentView, AppError> {
    let agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let is_owner = requesting_user_id.map_or(false, |uid| uid == agent.user_id);

    Ok(AgentView {
        agent_id: agent.agent_id,
        user_id: agent.user_id,
        wallet_address: agent.wallet_address.clone(),
        name: agent.name.clone(),
        description: agent.description.clone(),
        webhook_url: if is_owner { agent.webhook_url.clone() } else { None },
        status: agent.status.clone(),
        created_at: agent.created_at,
        updated_at: agent.updated_at,
        last_seen_at: agent.last_seen_at,
    })
}

pub async fn list_agents(user_id: Uuid, state: &AppState) -> Result<Vec<Agent>, AppError> {
    state.agent_store.list_agents_by_user(user_id).await
}

pub async fn update_agent(
    agent_id: Uuid,
    user_id: Uuid,
    req: UpdateAgentRequest,
    state: &AppState,
) -> Result<Agent, AppError> {
    let mut agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if agent.user_id != user_id {
        return Err(AppError::Forbidden("Not your agent".to_string()));
    }

    if let Some(name) = req.name {
        if name.is_empty() || name.len() > 32 {
            return Err(AppError::BadRequest("Name must be 1-32 chars".to_string()));
        }
        agent.name = name;
    }
    if let Some(desc) = req.description {
        if desc.len() > 256 {
            return Err(AppError::BadRequest("Description max 256 chars".to_string()));
        }
        agent.description = Some(desc);
    }
    if let Some(url) = req.webhook_url {
        if url.len() > 512 || !is_valid_url(&url) {
            return Err(AppError::BadRequest("Invalid webhook URL".to_string()));
        }
        agent.webhook_url = Some(url);
    }
    if let Some(status) = req.status {
        agent.status = status;
    }
    agent.updated_at = Utc::now();

    state.agent_store.update_agent(&agent).await
}

pub async fn rotate_api_key(
    agent_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> Result<String, AppError> {
    let mut agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if agent.user_id != user_id {
        return Err(AppError::Forbidden("Not your agent".to_string()));
    }

    let raw_key = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        format!("bth_{}", hex::encode(bytes))
    };

    let key_hash =
        bcrypt::hash(&raw_key, state.config.bcrypt_cost).map_err(|e| AppError::Internal(e.into()))?;

    agent.api_key_hash = key_hash;
    agent.updated_at = Utc::now();
    state.agent_store.update_agent(&agent).await?;

    let key_prefix = &raw_key[4..20];
    state
        .cache
        .set_agent_key(key_prefix, &agent.agent_id.to_string())
        .await?;

    Ok(raw_key)
}

pub async fn get_stats(agent_id: Uuid, state: &AppState) -> Result<Vec<AgentStats>, AppError> {
    state.agent_store.get_stats(agent_id).await
}

pub async fn get_leaderboard(
    game_type: Option<String>,
    sort_by: String,
    _period: String,
    limit: i64,
    offset: i64,
    state: &AppState,
) -> Result<Vec<LeaderboardEntry>, AppError> {
    let valid_sorts = ["win_rate", "net_profit_wei", "games_played"];
    if !valid_sorts.contains(&sort_by.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid sort_by: {}", sort_by)));
    }

    let limit = limit.min(100);

    let pairs = state
        .agent_store
        .get_leaderboard(game_type.as_deref(), &sort_by, limit, offset)
        .await?;

    Ok(pairs
        .into_iter()
        .enumerate()
        .map(|(i, (agent, stats))| LeaderboardEntry {
            rank: offset as usize + i + 1,
            agent: agent.into(),
            stats,
        })
        .collect())
}

pub async fn authenticate_agent_key(
    api_key: &str,
    state: &AppState,
) -> Result<Agent, AppError> {
    if !api_key.starts_with("bth_") || api_key.len() != 68 {
        return Err(AppError::Unauthorized("Invalid API key format".to_string()));
    }

    let key_prefix = &api_key[4..20];
    let agent_id_str = state
        .cache
        .get_agent_by_key(key_prefix)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;

    let agent_id: Uuid = agent_id_str
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid agent ID".to_string()))?;

    let agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Agent not found".to_string()))?;

    if agent.status == AgentStatus::Suspended {
        return Err(AppError::Forbidden("AGENT_SUSPENDED".to_string()));
    }

    let valid = bcrypt::verify(api_key, &agent.api_key_hash)
        .map_err(|e| AppError::Internal(e.into()))?;
    if !valid {
        return Err(AppError::Unauthorized("Invalid API key".to_string()));
    }

    state.agent_store.update_last_seen(agent_id).await?;
    Ok(agent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::auth_store::MemoryAuthStore;
    use crate::adapters::memory::cache_store::MemoryCacheStore;
    use crate::adapters::memory::agent_store::MemoryAgentStore;
    use crate::adapters::memory::analytics_store::MemoryAnalyticsStore;
    use crate::adapters::memory::lobby_store::MemoryLobbyStore;
    use crate::adapters::memory::game_store::MemoryGameStore;
    use crate::adapters::memory::event_bus::MemoryEventBus;
    use crate::config::Config;
    use crate::games::GameRegistry;
    use std::sync::Arc;
    use tokio::sync::watch;

    fn make_state() -> AppState {
        let config = Config {
            database_url: "".to_string(),
            redis_url: "".to_string(),
            jwt_secret: "test_secret".to_string(),
            jwt_expiry_secs: 86400,
            refresh_token_expiry_secs: 2592000,
            bcrypt_cost: 4,
            house_signing_key: "deadbeef".to_string(),
            turn_timeout_ms: 10000,
            settlement_rpc_url: "".to_string(),
            settlement_private_key: "".to_string(),
            escrow_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
            house_wallet_address: "0x0000000000000000000000000000000000000000".to_string(),
            chain_id: 84532,
            rake_bps: 500,
            port: 8080,
            cors_origins: vec![],
            base_url: "http://localhost:8080".to_string(),
            testnet_base_url: "http://localhost:8080".to_string(),
        };

        struct NoopSettlement;
        #[async_trait::async_trait]
        impl crate::ports::settlement_port::SettlementPort for NoopSettlement {
            async fn settle(&self, _: Uuid, _: &[crate::domain::game::WinnerEntry], _: &str, _: &str) -> Result<String, AppError> { Ok("0x".to_string()) }
            async fn check_confirmation(&self, _: &str) -> Result<Option<i64>, AppError> { Ok(None) }
            async fn check_escrow_deposit(&self, _: Uuid, _: &str, _: &str) -> Result<bool, AppError> { Ok(true) }
        }

        struct NoopHttpClient;
        #[async_trait::async_trait]
        impl crate::ports::http_client::HttpClient for NoopHttpClient {
            async fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<u16, AppError> { Ok(200) }
        }

        let (shutdown_tx, _) = watch::channel(());
        AppState {
            auth_store: Arc::new(MemoryAuthStore::new()),
            agent_store: Arc::new(MemoryAgentStore::new()),
            analytics_store: Arc::new(MemoryAnalyticsStore::new()),
            game_store: Arc::new(MemoryGameStore::new()),
            lobby_store: Arc::new(MemoryLobbyStore::new()),
            cache: Arc::new(MemoryCacheStore::new()),
            event_bus: Arc::new(MemoryEventBus::new()),
            settlement: Arc::new(NoopSettlement),
            http_client: Arc::new(NoopHttpClient),
            game_registry: Arc::new(GameRegistry::new()),
            config: Arc::new(config),
            shutdown_tx: Arc::new(shutdown_tx),
        }
    }

    #[tokio::test]
    async fn test_register_agent() {
        let state = make_state();
        let user_id = Uuid::new_v4();
        let req = RegisterAgentRequest {
            name: "TestAgent".to_string(),
            wallet_address: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            description: None,
            webhook_url: None,
        };
        let (agent, raw_key) = register_agent(user_id, req, &state).await.unwrap();
        assert_eq!(agent.user_id, user_id);
        assert!(raw_key.starts_with("bth_"));
        assert_eq!(raw_key.len(), 68);
    }

    #[tokio::test]
    async fn test_duplicate_wallet_rejected() {
        let state = make_state();
        let user_id = Uuid::new_v4();
        let req1 = RegisterAgentRequest {
            name: "Agent1".to_string(),
            wallet_address: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            description: None,
            webhook_url: None,
        };
        register_agent(user_id, req1, &state).await.unwrap();
        let req2 = RegisterAgentRequest {
            name: "Agent2".to_string(),
            wallet_address: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            description: None,
            webhook_url: None,
        };
        let result = register_agent(user_id, req2, &state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_api_key_auth() {
        let state = make_state();
        let user_id = Uuid::new_v4();
        let req = RegisterAgentRequest {
            name: "KeyAgent".to_string(),
            wallet_address: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            description: None,
            webhook_url: None,
        };
        let (agent, raw_key) = register_agent(user_id, req, &state).await.unwrap();
        let authed = authenticate_agent_key(&raw_key, &state).await.unwrap();
        assert_eq!(authed.agent_id, agent.agent_id);
    }
}
