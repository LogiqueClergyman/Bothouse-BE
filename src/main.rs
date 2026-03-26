use bothouse_backend::adapters::ethereum::settlement::EthereumSettlement;
use bothouse_backend::adapters::redis::cache_store::RedisCache;
use bothouse_backend::adapters::redis::event_bus::RedisEventBus;
use bothouse_backend::adapters::postgres::agent_store::PgAgentStore;
use bothouse_backend::adapters::postgres::analytics_store::PgAnalyticsStore;
use bothouse_backend::adapters::postgres::auth_store::PgAuthStore;
use bothouse_backend::adapters::postgres::game_store::PgGameStore;
use bothouse_backend::adapters::postgres::lobby_store::PgLobbyStore;
use bothouse_backend::adapters::reqwest::http_client::ReqwestHttpClient;
use bothouse_backend::config::Config;
use bothouse_backend::games::texas_holdem_v1::engine::TexasHoldemV1;
use bothouse_backend::games::GameRegistry;
use bothouse_backend::state::AppState;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load .env
    let _ = dotenvy::dotenv();

    // 2. Initialize tracing
    let node_env = std::env::var("NODE_ENV").unwrap_or_default();
    if node_env == "production" {
        tracing_subscriber::fmt()
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .pretty()
            .init();
    }

    // 3. Parse config
    let config = Config::from_env()?;
    let port = config.port;

    // 4. Connect to PostgreSQL
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    // 5. Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // 6. Connect to Redis
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client.clone()).await?;

    // 7. Construct adapters
    let auth_store = Arc::new(PgAuthStore::new(pool.clone()));
    let agent_store = Arc::new(PgAgentStore::new(pool.clone()));
    let analytics_store = Arc::new(PgAnalyticsStore::new(pool.clone()));
    let game_store = Arc::new(PgGameStore::new(pool.clone()));
    let lobby_store = Arc::new(PgLobbyStore::new(pool.clone()));
    let cache = Arc::new(RedisCache::new(redis_conn.clone()));
    let event_bus = Arc::new(RedisEventBus::new(redis_client, redis_conn));
    let settlement = Arc::new(EthereumSettlement::new(
        config.settlement_rpc_url.clone(),
        config.settlement_private_key.clone(),
        config.escrow_contract_address.clone(),
        config.chain_id,
    ));
    let http_client = Arc::new(ReqwestHttpClient::new());

    // 8. Build GameRegistry
    let mut registry = GameRegistry::new();
    registry.register(Box::new(TexasHoldemV1));

    // 9. Shutdown channel
    let (shutdown_tx, _shutdown_rx) = watch::channel(());

    // 10. Build AppState
    let state = AppState {
        auth_store,
        agent_store,
        analytics_store,
        game_store,
        lobby_store,
        cache,
        event_bus,
        settlement,
        http_client,
        game_registry: Arc::new(registry),
        config: Arc::new(config),
        shutdown_tx: Arc::new(shutdown_tx),
    };

    // 11. Build router
    let app = bothouse_backend::api::router::build(state.clone());

    // 12. Bind
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    // 13. Log startup
    tracing::info!("BotTheHouse backend started on port {}", port);

    // 14. Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state))
        .await?;

    Ok(())
}

async fn shutdown_signal(state: AppState) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, shutting down turn managers...");
    let _ = state.shutdown_tx.send(());
}
