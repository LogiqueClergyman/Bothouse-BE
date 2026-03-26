use axum::routing::{get, post, put};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::api::handlers::{agents, analytics, auth, games, lobby, manifest, settlement};
use crate::state::AppState;

pub fn build(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        // Auth
        .route("/auth/nonce", get(auth::get_nonce))
        .route("/auth/verify", post(auth::verify_signature))
        .route("/auth/refresh", post(auth::refresh_token))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::get_me))
        // Agents
        .route("/agents/register", post(agents::register_agent))
        .route("/agents", get(agents::list_agents))
        .route("/agents/leaderboard", get(agents::get_leaderboard))
        .route("/agents/:agent_id", get(agents::get_agent))
        .route("/agents/:agent_id", put(agents::update_agent))
        .route("/agents/:agent_id/rotate-key", post(agents::rotate_key))
        .route("/agents/:agent_id/stats", get(agents::get_stats))
        // Analytics
        .route("/agents/:agent_id/tendencies", get(analytics::get_tendencies))
        .route("/agents/:agent_id/actions", get(analytics::list_actions))
        .route("/agents/:agent_id/hands", get(analytics::list_hands))
        .route("/agents/:agent_id/vs/:opponent_id", get(analytics::get_head_to_head))
        // Lobby
        .route("/lobby/rooms", get(lobby::list_rooms))
        .route("/lobby/rooms", post(lobby::create_room))
        .route("/lobby/rooms/:room_id", get(lobby::get_room))
        .route("/lobby/rooms/:room_id/join", post(lobby::join_room))
        .route("/lobby/rooms/:room_id/leave", post(lobby::leave_room))
        .route("/lobby/join-queue", post(lobby::join_queue))
        // Games
        .route("/games", get(games::list_games))
        .route("/games/:game_id", get(games::get_game))
        .route("/games/:game_id/spectate", get(games::spectate_game))
        .route("/games/:game_id/state", get(games::get_game_state))
        .route("/games/:game_id/action", post(games::submit_action))
        .route("/games/:game_id/log", get(games::get_game_log))
        // Settlement
        .route("/settle/:game_id", get(settlement::get_settlement))
        .route("/settle/agent/:agent_id/history", get(settlement::get_agent_history))
        // Platform stats
        .route("/stats", get(manifest::get_platform_stats))
        // OpenAPI spec
        .merge(crate::api::openapi::router());

    Router::new()
        .nest("/api/v1", api)
        .route("/agent-manifest.json", get(manifest::get_manifest))
        .route("/health", get(health_check))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "1.0.0",
    }))
}
