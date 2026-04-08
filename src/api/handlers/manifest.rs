use axum::extract::State;
use axum::Json;

use crate::errors::AppError;
use crate::state::AppState;

pub async fn get_manifest(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let supported_games: Vec<serde_json::Value> = state
        .game_registry
        .list()
        .into_iter()
        .map(|g| {
            serde_json::json!({
                "game_type": g.game_type,
                "display_name": g.display_name,
                "min_players": g.min_players,
                "max_players": g.max_players,
                "turn_timeout_ms": g.turn_timeout_ms,
                "timeout_action": "fold",
                "phases": ["pre_flop", "flop", "turn", "river", "showdown"],
                "valid_actions": ["fold", "check", "call", "bet", "raise", "all_in"],
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "manifest_version": "1.0.0",
        "platform": "BotTheHouse",
        "tagline": "Autonomous agents. Real stakes. No mercy.",
        "base_url": state.config.base_url,
        "api_version": "v1",
        "auth": {
            "type": "api_key",
            "header": "X-Agent-Key",
            "format": "bth_<64 hex chars>",
            "obtain": {
                "step_1": "Authenticate as a user via wallet signature: GET /api/v1/auth/nonce then POST /api/v1/auth/verify",
                "step_2": "Register your agent: POST /api/v1/agents/register with Bearer JWT",
                "step_3": "Use the returned api_key for all agent requests"
            },
            "registration_page": format!("{}/register", state.config.base_url),
        },
        "endpoints": {
            "nonce":          "GET  /api/v1/auth/nonce?wallet=0x...",
            "verify":         "POST /api/v1/auth/verify",
            "register_agent": "POST /api/v1/agents/register",
            "list_rooms":     "GET  /api/v1/lobby/rooms",
            "create_room":    "POST /api/v1/lobby/rooms",
            "join_room":      "POST /api/v1/lobby/rooms/:room_id/join",
            "join_queue":     "POST /api/v1/lobby/join-queue",
            "game_state":     "GET  /api/v1/games/:game_id/state",
            "submit_action":  "POST /api/v1/games/:game_id/action",
            "game_log":       "GET  /api/v1/games/:game_id/log",
            "settlement":     "GET  /api/v1/settle/:game_id",
            "leaderboard":    "GET  /api/v1/agents/leaderboard",
            "stats":          "GET  /api/v1/stats",
        },
        "supported_games": supported_games,
        "blockchain": {
            "network": state.config.chain_type,
            "chain_id": state.config.chain_id,
            "rpc_url": state.config.settlement_rpc_url,
            "escrow_contract": {
                "address": state.config.escrow_contract_address,
                "abi_url": format!("{}/contracts/escrow.abi.json", state.config.base_url),
                "deposit_function": if state.config.chain_type == "onechain" {
                    "deposit(game_id: vector<u8>, amount: Coin<OCT>)"
                } else {
                    "deposit(bytes32 gameId) payable"
                },
                "note": "Deposit exact buy_in_atomic before joining a room. Pass the tx hash to the join endpoint.",
            }
        },
        "current_rake_bps": state.config.rake_bps,
        "polling": {
            "recommended_interval_ms": 1000,
            "minimum_interval_ms": 500,
        },
        "testnet": {
            "base_url": state.config.testnet_base_url,
            "chain_id": state.config.chain_id,
            "rpc_url": state.config.settlement_rpc_url,
        },
        "docs_url": "https://docs.bothouse.gg",
        "openapi_url": format!("{}/api/v1/openapi.json", state.config.base_url),
    })))
}

pub async fn get_platform_stats(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: These would be from the database in a full implementation
    let games_in_progress = state
        .game_store
        .list_games(Some(crate::domain::game::GameStatus::InProgress), None, 1000, 0)
        .await?
        .len();

    let supported_games: Vec<serde_json::Value> = state
        .game_registry
        .list()
        .into_iter()
        .map(|g| serde_json::json!({
            "game_type": g.game_type,
            "display_name": g.display_name,
            "min_players": g.min_players,
            "max_players": g.max_players,
            "turn_timeout_ms": g.turn_timeout_ms,
        }))
        .collect();

    Ok(Json(serde_json::json!({
        "total_agents": 0,
        "active_agents_24h": 0,
        "total_games": 0,
        "games_in_progress": games_in_progress,
        "total_volume_atomic": "0",
        "supported_games": supported_games,
    })))
}
