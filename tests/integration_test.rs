use std::sync::Arc;
use axum::http::{Request, StatusCode};
use axum::body::Body;
use tower::util::ServiceExt;
use tokio::sync::watch;

use bothouse_backend::adapters::memory::{
    agent_store::MemoryAgentStore,
    auth_store::MemoryAuthStore,
    cache_store::MemoryCacheStore,
    event_bus::MemoryEventBus,
    game_store::MemoryGameStore,
    lobby_store::MemoryLobbyStore,
};
use bothouse_backend::config::Config;
use bothouse_backend::errors::AppError;
use bothouse_backend::games::GameRegistry;
use bothouse_backend::games::texas_holdem_v1::engine::TexasHoldemV1;
use bothouse_backend::ports::settlement_port::SettlementPort;
use bothouse_backend::ports::http_client::HttpClient;
use bothouse_backend::state::AppState;
use bothouse_backend::domain::game::WinnerEntry;
use uuid::Uuid;

struct NoopSettlement;

#[async_trait::async_trait]
impl SettlementPort for NoopSettlement {
    async fn settle(
        &self,
        _game_id: Uuid,
        _winners: &[WinnerEntry],
        _rake_wei: &str,
        _result_hash: &str,
    ) -> Result<String, AppError> {
        Ok("0xdeadbeef".to_string())
    }

    async fn check_confirmation(&self, _tx_hash: &str) -> Result<Option<i64>, AppError> {
        Ok(Some(12345))
    }

    async fn check_escrow_deposit(
        &self,
        _game_id: Uuid,
        _wallet: &str,
        _buy_in_wei: &str,
    ) -> Result<bool, AppError> {
        Ok(true)
    }
}

struct NoopHttpClient;

#[async_trait::async_trait]
impl HttpClient for NoopHttpClient {
    async fn post_json(&self, _url: &str, _body: &serde_json::Value) -> Result<u16, AppError> {
        Ok(200)
    }
}

fn make_test_state() -> AppState {
    let config = Config {
        database_url: "postgres://localhost/test".to_string(),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: "test_secret_that_is_at_least_32_bytes_long!".to_string(),
        jwt_expiry_secs: 86400,
        refresh_token_expiry_secs: 2592000,
        bcrypt_cost: 4,
        house_signing_key: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        turn_timeout_ms: 5000,
        settlement_rpc_url: "https://sepolia.base.org".to_string(),
        settlement_private_key: "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        escrow_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
        house_wallet_address: "0x0000000000000000000000000000000000000000".to_string(),
        chain_id: 84532,
        rake_bps: 500,
        port: 8080,
        cors_origins: vec![],
        base_url: "http://localhost:8080".to_string(),
        testnet_base_url: "http://localhost:8080".to_string(),
    };

    let mut registry = GameRegistry::new();
    registry.register(Box::new(TexasHoldemV1));

    let (shutdown_tx, _) = watch::channel(());

    AppState {
        auth_store: Arc::new(MemoryAuthStore::new()),
        agent_store: Arc::new(MemoryAgentStore::new()),
        game_store: Arc::new(MemoryGameStore::new()),
        lobby_store: Arc::new(MemoryLobbyStore::new()),
        cache: Arc::new(MemoryCacheStore::new()),
        event_bus: Arc::new(MemoryEventBus::new()),
        settlement: Arc::new(NoopSettlement),
        http_client: Arc::new(NoopHttpClient),
        game_registry: Arc::new(registry),
        config: Arc::new(config),
        shutdown_tx: Arc::new(shutdown_tx),
    }
}

async fn body_json(body: Body) -> serde_json::Value {
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn build_test_router() -> axum::Router {
    let state = make_test_state();
    bothouse_backend::api::router::build(state)
}

// --- Health check ---

#[tokio::test]
async fn test_health_check() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// --- Manifest endpoint ---

#[tokio::test]
async fn test_agent_manifest() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/agent-manifest.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json.get("api_version").is_some(), "manifest should have api_version");
    assert!(json.get("supported_games").is_some(), "manifest should have supported_games");
}

// --- Auth endpoints ---

#[tokio::test]
async fn test_get_nonce_valid_wallet() {
    let app = build_test_router();
    let wallet = "0xabcdef1234567890abcdef1234567890abcdef12";
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/auth/nonce?wallet={}", wallet))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json.get("nonce").is_some());
    assert!(json.get("expires_at").is_some());
    let nonce = json["nonce"].as_str().unwrap();
    assert_eq!(nonce.len(), 64, "nonce should be 64 hex chars");
}

#[tokio::test]
async fn test_get_nonce_invalid_wallet() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/nonce?wallet=not_a_wallet")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert!(res.status().is_client_error());
}

// --- Lobby endpoints ---

#[tokio::test]
async fn test_list_rooms_empty() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/lobby/rooms")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json.get("rooms").is_some());
    assert_eq!(json["rooms"].as_array().unwrap().len(), 0);
}

// --- Games endpoints ---

#[tokio::test]
async fn test_list_games_empty() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/games")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json.get("games").is_some());
}

#[tokio::test]
async fn test_get_game_not_found() {
    let app = build_test_router();
    let fake_id = Uuid::new_v4();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/games/{}", fake_id))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// --- Agent endpoints ---

#[tokio::test]
async fn test_get_leaderboard() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/agents/leaderboard")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json.get("leaderboard").is_some());
}

#[tokio::test]
async fn test_get_agent_not_found() {
    let app = build_test_router();
    let fake_id = Uuid::new_v4();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/agents/{}", fake_id))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// --- Settlement endpoints ---

#[tokio::test]
async fn test_get_settlement_not_found() {
    let app = build_test_router();
    let fake_id = Uuid::new_v4();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/settle/{}", fake_id))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// --- Register agent (requires auth) ---

#[tokio::test]
async fn test_register_agent_requires_auth() {
    let app = build_test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/agents/register")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"TestBot","wallet_address":"0xabcdef1234567890abcdef1234567890abcdef12"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// --- Create room (requires agent API key) ---

#[tokio::test]
async fn test_create_room_requires_agent_key() {
    let app = build_test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/lobby/rooms")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"game_type":"texas_holdem_v1","buy_in_wei":"1000000000000000000","max_players":6,"min_players":2,"escrow_tx_hash":"0xabc"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// --- Stats endpoint ---

#[tokio::test]
async fn test_platform_stats() {
    let app = build_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/stats")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json.get("total_agents").is_some());
    assert!(json.get("games_in_progress").is_some());
    assert!(json.get("total_volume_wei").is_some());
}
