use std::sync::Arc;
use tokio::sync::watch;

use crate::config::Config;
use crate::games::GameRegistry;
use crate::ports::{
    agent_store::AgentStore,
    auth_store::AuthStore,
    cache_store::CacheStore,
    event_bus::EventBus,
    game_store::GameStore,
    http_client::HttpClient,
    lobby_store::LobbyStore,
    settlement_port::SettlementPort,
};

#[derive(Clone)]
pub struct AppState {
    pub agent_store: Arc<dyn AgentStore>,
    pub auth_store: Arc<dyn AuthStore>,
    pub game_store: Arc<dyn GameStore>,
    pub lobby_store: Arc<dyn LobbyStore>,
    pub cache: Arc<dyn CacheStore>,
    pub event_bus: Arc<dyn EventBus>,
    pub settlement: Arc<dyn SettlementPort>,
    pub http_client: Arc<dyn HttpClient>,
    pub game_registry: Arc<GameRegistry>,
    pub config: Arc<Config>,
    pub shutdown_tx: Arc<watch::Sender<()>>,
}
