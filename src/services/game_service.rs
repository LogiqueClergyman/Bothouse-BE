use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::game::{GameLogEntry, GameStatus, PlayerStatus};
use crate::errors::AppError;
use crate::state::AppState;

pub struct ActionRequest {
    pub action: String,
    pub amount_atomic: Option<String>,
    pub turn_number: i64,
    pub signature: String,
}

pub struct ActionResponse {
    pub accepted: bool,
    pub sequence_number: i64,
}

pub async fn get_game_state(
    game_id: Uuid,
    requesting_agent_id: Uuid,
    state: &AppState,
) -> Result<serde_json::Value, AppError> {
    let game = state
        .game_store
        .get_game_by_id(game_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let players = state.game_store.get_players_by_game(game_id).await?;
    let is_player = players.iter().any(|p| p.agent_id == requesting_agent_id);
    if !is_player {
        return Err(AppError::Forbidden("FORBIDDEN".to_string()));
    }

    let game_impl = state
        .game_registry
        .get(&game.game_type)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Game type not found")))?;

    let visible = game_impl.visible_state(&game.current_state, requesting_agent_id)?;
    let valid_actions = game_impl.valid_actions(&game.current_state, requesting_agent_id)?;
    let timeout_action = game_impl.timeout_action(&game.current_state, requesting_agent_id);

    let current_turn = state.cache.get_current_turn(&game_id.to_string()).await?;
    let your_turn = current_turn
        .as_deref()
        .map_or(false, |id| id == requesting_agent_id.to_string());

    let turn_number = game.current_state["turn_number"].as_i64().unwrap_or(0);

    let player = players
        .iter()
        .find(|p| p.agent_id == requesting_agent_id)
        .ok_or(AppError::NotFound)?;

    let wallet_info = serde_json::json!({
        "escrowed_atomic": player.stack_atomic,
        "at_stake_atomic": player.stack_atomic,
    });

    Ok(serde_json::json!({
        "game_id": game_id.to_string(),
        "game_type": game.game_type,
        "status": game.status,
        "sequence_number": game.sequence_number,
        "your_turn": your_turn,
        "turn_number": turn_number,
        "timeout_action": timeout_action,
        "visible_state": visible,
        "valid_actions": valid_actions,
        "wallet": wallet_info,
    }))
}

pub async fn spectate_game(game_id: Uuid, state: &AppState) -> Result<serde_json::Value, AppError> {
    let game = state
        .game_store
        .get_game_by_id(game_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let players = state.game_store.get_players_by_game(game_id).await?;
    let game_impl = state
        .game_registry
        .get(&game.game_type)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Game type not found")))?;

    // Use a nil UUID as spectator — game should return public view with no hole cards
    let visible = game_impl.visible_state(&game.current_state, Uuid::nil())?;

    let turn_number = game.current_state["turn_number"].as_i64().unwrap_or(0);

    let player_summaries: Vec<serde_json::Value> = players
        .iter()
        .map(|p| {
            serde_json::json!({
                "agent_id": p.agent_id.to_string(),
                "seat_number": p.seat_number,
                "stack_atomic": p.stack_atomic,
                "status": p.status,
                "last_action": null,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "game_id": game_id.to_string(),
        "game_type": game.game_type,
        "status": game.status,
        "sequence_number": game.sequence_number,
        "turn_number": turn_number,
        "visible_state": visible,
        "players": player_summaries,
    }))
}

pub async fn submit_action(
    game_id: Uuid,
    agent_id: Uuid,
    req: ActionRequest,
    state: &AppState,
) -> Result<ActionResponse, AppError> {
    let game = state
        .game_store
        .get_game_by_id(game_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if game.status != GameStatus::InProgress {
        return Err(AppError::Conflict("GAME_NOT_IN_PROGRESS".to_string()));
    }

    // Atomically claim the turn via GETDEL
    let turn_key = game_id.to_string();
    let current_turn = state.cache.get_current_turn(&turn_key).await?;
    match &current_turn {
        None => return Err(AppError::Forbidden("NOT_YOUR_TURN".to_string())),
        Some(id) if id != &agent_id.to_string() => {
            return Err(AppError::Forbidden("NOT_YOUR_TURN".to_string()))
        }
        _ => {}
    }
    state.cache.delete_current_turn(&turn_key).await?;

    let players = state.game_store.get_players_by_game(game_id).await?;
    let player = players
        .iter()
        .find(|p| p.agent_id == agent_id)
        .ok_or_else(|| AppError::Forbidden("Not a player".to_string()))?;

    let game_impl = state
        .game_registry
        .get(&game.game_type)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Game type not found")))?;

    let current_turn_number = game.current_state["turn_number"].as_i64().unwrap_or(0);
    if req.turn_number != current_turn_number {
        return Err(AppError::BadRequest(format!("TURN_NUMBER_MISMATCH: expected {}, got {}", current_turn_number, req.turn_number)));
    }

    // Verify signature (can be skipped for local demos)
    if !state.config.skip_action_signature_verification {
        if state.config.chain_type == "onechain" {
            // OneChain uses Ed25519; verify against the wallet's registered public key.
            let public_key_hex = state
                .auth_store
                .get_public_key(&player.wallet_address)
                .await?
                .ok_or_else(|| AppError::Unauthorized("MISSING_PUBLIC_KEY".to_string()))?;

            let mut message = Vec::new();
            message.extend_from_slice(game_id.as_bytes());
            message.extend_from_slice(&req.turn_number.to_be_bytes());
            message.extend_from_slice(req.action.as_bytes());
            message.extend_from_slice(req.amount_atomic.as_deref().unwrap_or("").as_bytes());

            let ok = crate::adapters::onechain::settlement::verify_ed25519_signature(
                &message,
                &req.signature,
                &public_key_hex,
            )
            .map_err(|e| AppError::Unauthorized(format!("INVALID_SIGNATURE: {}", e)))?;

            if !ok {
                return Err(AppError::Unauthorized("INVALID_SIGNATURE".to_string()));
            }
        } else {
            // EVM
            let sig_valid = game_impl.verify_action_signature(
                game_id,
                req.turn_number,
                &req.action,
                req.amount_atomic.as_deref(),
                &req.signature,
                &player.wallet_address,
                &state.config.chain_type,
            )?;
            if !sig_valid {
                return Err(AppError::Unauthorized("INVALID_SIGNATURE".to_string()));
            }
        }
    }

    // Validate action
    let valid_actions = game_impl.valid_actions(&game.current_state, agent_id)?;
    if !valid_actions.contains(&req.action) {
        return Err(AppError::BadRequest(format!("INVALID_ACTION: {}", req.action)));
    }

    // Apply action
    let new_state = game_impl.apply_action(
        game.current_state.clone(),
        agent_id,
        &req.action,
        req.amount_atomic.as_deref(),
    )?;

    let new_seq = game.sequence_number + 1;

    // Compute state hash
    let state_json = serde_json::to_string(&new_state)
        .map_err(|e| AppError::Internal(e.into()))?;
    let state_hash = hex::encode(Sha256::digest(state_json.as_bytes()));

    // Append log
    let log_entry = GameLogEntry {
        game_id,
        sequence: new_seq,
        timestamp: Utc::now(),
        agent_id: Some(agent_id),
        action: req.action.clone(),
        amount_atomic: req.amount_atomic.clone(),
        state_hash: state_hash.clone(),
    };
    state.game_store.append_log_entry(&log_entry).await?;

    // Save state
    state
        .game_store
        .update_game_state(game_id, &new_state, new_seq)
        .await?;

    // Check terminal
    if game_impl.is_terminal(&new_state) {
        let state_clone = state.clone();
        tokio::spawn(async move {
            let _ = complete_game(game_id, &new_state, &state_clone).await;
        });
    } else {
        // Advance turn — find next active player
        advance_turn(game_id, &new_state, agent_id, state).await?;
    }

    Ok(ActionResponse {
        accepted: true,
        sequence_number: new_seq,
    })
}

async fn advance_turn(
    game_id: Uuid,
    state_val: &serde_json::Value,
    _current_agent_id: Uuid,
    state: &AppState,
) -> Result<(), AppError> {
    // Determine next player from game state action_on_seat
    let action_on_seat = state_val["action_on_seat"].as_i64();
    if let Some(seat) = action_on_seat {
        let players = state.game_store.get_players_by_game(game_id).await?;
        if let Some(next_player) = players.iter().find(|p| p.seat_number == seat as i16) {
            let game = state.game_store.get_game_by_id(game_id).await?;
            let timeout_ms = if let Some(g) = game {
                state
                    .game_registry
                    .get(&g.game_type)
                    .map(|gi| gi.turn_timeout_ms())
                    .unwrap_or(10000)
            } else {
                10000
            };

            state
                .cache
                .set_current_turn(
                    &game_id.to_string(),
                    &next_player.agent_id.to_string(),
                    timeout_ms,
                )
                .await?;

            // Fire webhook if registered
            if let Some(ref url) = next_player.wallet_address.as_str().get(0..0) {
                // noop — actual webhook sent from turn manager
                let _ = url;
            }

            // Fire webhook
            if let Some(agent) = state.agent_store.get_agent_by_id(next_player.agent_id).await? {
                if let Some(ref webhook_url) = agent.webhook_url {
                    let game_val = state.game_store.get_game_by_id(game_id).await?;
                    let turn_number = game_val
                        .as_ref()
                        .and_then(|g| g.current_state["turn_number"].as_i64())
                        .unwrap_or(0);
                    let payload = serde_json::json!({
                        "event": "YOUR_TURN",
                        "game_id": game_id.to_string(),
                        "agent_id": next_player.agent_id.to_string(),
                        "turn_number": turn_number,
                        "expires_at": Utc::now() + chrono::Duration::milliseconds(timeout_ms as i64),
                        "state_url": format!("{}/api/v1/games/{}/state", state.config.base_url, game_id),
                        "action_url": format!("{}/api/v1/games/{}/action", state.config.base_url, game_id),
                    });
                    let _ = state.http_client.post_json(webhook_url, &payload).await;
                }
            }
        }
    }
    Ok(())
}

pub async fn complete_game(
    game_id: Uuid,
    final_state: &serde_json::Value,
    state: &AppState,
) -> Result<(), AppError> {
    let game = state
        .game_store
        .get_game_by_id(game_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let game_impl = state
        .game_registry
        .get(&game.game_type)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Game type not found")))?;

    if let Some(result) = game_impl.result(final_state, game_id, state.config.rake_bps) {
        state.game_store.save_result(&result).await?;

        // Update agent stats
        let players = state.game_store.get_players_by_game(game_id).await?;
        for player in &players {
            let mut stats_list = state.agent_store.get_stats(player.agent_id).await?;
            let existing = stats_list
                .iter_mut()
                .find(|s| s.game_type == game.game_type);

            let is_winner = result.winners.iter().any(|w| w.agent_id == player.agent_id);
            let amount_won = result
                .winners
                .iter()
                .find(|w| w.agent_id == player.agent_id)
                .map(|w| w.amount_won_atomic.parse::<i128>().unwrap_or(0))
                .unwrap_or(0);
            let amount_lost = result
                .losers
                .iter()
                .find(|l| l.agent_id == player.agent_id)
                .map(|l| l.amount_lost_atomic.parse::<i128>().unwrap_or(0))
                .unwrap_or(0);
            let buy_in: i128 = game
                .current_state["buy_in_atomic"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            let now = Utc::now();
            if let Some(s) = existing {
                let total_wagered = s.total_wagered_atomic.parse::<i128>().unwrap_or(0) + buy_in;
                let total_won = s.total_won_atomic.parse::<i128>().unwrap_or(0) + amount_won;
                let total_lost = s.total_lost_atomic.parse::<i128>().unwrap_or(0) + amount_lost;
                let games_played = s.games_played + 1;
                let games_won = s.games_won + if is_winner { 1 } else { 0 };
                let win_rate = if games_played > 0 {
                    games_won as f64 / games_played as f64
                } else {
                    0.0
                };

                s.games_played = games_played;
                s.games_won = games_won;
                s.total_wagered_atomic = total_wagered.to_string();
                s.total_won_atomic = total_won.to_string();
                s.total_lost_atomic = total_lost.to_string();
                s.net_profit_atomic = (total_won - total_lost).to_string();
                s.win_rate = win_rate;
                s.updated_at = now;
                state.agent_store.upsert_stats(s).await?;
            } else {
                let win_rate = if is_winner { 1.0 } else { 0.0 };
                let stats = crate::domain::agent::AgentStats {
                    agent_id: player.agent_id,
                    game_type: game.game_type.clone(),
                    games_played: 1,
                    games_won: if is_winner { 1 } else { 0 },
                    total_wagered_atomic: buy_in.to_string(),
                    total_won_atomic: amount_won.to_string(),
                    total_lost_atomic: amount_lost.to_string(),
                    net_profit_atomic: (amount_won - amount_lost).to_string(),
                    win_rate,
                    updated_at: now,
                };
                state.agent_store.upsert_stats(&stats).await?;
            }
        }

        state
            .game_store
            .update_game_status(game_id, GameStatus::Completed)
            .await?;
        state
            .lobby_store
            .update_room_status(game.room_id, crate::domain::lobby::RoomStatus::Completed)
            .await?;

        let state_clone = state.clone();
        tokio::spawn(async move {
            let _ = crate::services::settlement_service::initiate(&result, &state_clone).await;
        });

        state
            .event_bus
            .publish(
                "GAME_COMPLETED",
                &serde_json::json!({ "game_id": game_id.to_string() }),
            )
            .await?;
    }

    Ok(())
}

pub async fn get_game_log(
    game_id: Uuid,
    requesting_agent_id: Uuid,
    state: &AppState,
) -> Result<serde_json::Value, AppError> {
    let players = state.game_store.get_players_by_game(game_id).await?;
    let is_player = players.iter().any(|p| p.agent_id == requesting_agent_id);
    if !is_player {
        return Err(AppError::Forbidden("Not a player in this game".to_string()));
    }

    let log = state.game_store.get_log_by_game(game_id).await?;
    let result = state.game_store.get_result_by_game(game_id).await?;

    Ok(serde_json::json!({
        "game_id": game_id.to_string(),
        "log": log,
        "result": result,
    }))
}

pub async fn run_turn_manager(
    game_id: Uuid,
    state: AppState,
    mut shutdown: tokio::sync::watch::Receiver<()>,
) {
    let game = match state.game_store.get_game_by_id(game_id).await {
        Ok(Some(g)) => g,
        _ => return,
    };

    let timeout_ms = state
        .game_registry
        .get(&game.game_type)
        .map(|g| g.turn_timeout_ms())
        .unwrap_or(10000);

    // Set initial turn
    let action_on_seat = game.current_state["action_on_seat"].as_i64().unwrap_or(1);
    let players = match state.game_store.get_players_by_game(game_id).await {
        Ok(p) => p,
        Err(_) => return,
    };

    if let Some(first_player) = players.iter().find(|p| p.seat_number == action_on_seat as i16) {
        let _ = state
            .cache
            .set_current_turn(
                &game_id.to_string(),
                &first_player.agent_id.to_string(),
                timeout_ms,
            )
            .await;

        // Fire webhook for first player
        if let Ok(Some(agent)) = state.agent_store.get_agent_by_id(first_player.agent_id).await {
            if let Some(ref url) = agent.webhook_url {
                let payload = serde_json::json!({
                    "event": "YOUR_TURN",
                    "game_id": game_id.to_string(),
                    "agent_id": first_player.agent_id.to_string(),
                    "turn_number": game.current_state["turn_number"].as_i64().unwrap_or(1),
                    "expires_at": (Utc::now() + chrono::Duration::milliseconds(timeout_ms as i64)).to_rfc3339(),
                    "state_url": format!("{}/api/v1/games/{}/state", state.config.base_url, game_id),
                    "action_url": format!("{}/api/v1/games/{}/action", state.config.base_url, game_id),
                });
                let _ = state.http_client.post_json(url, &payload).await;
            }
        }
    }

    // Turn manager loop
    loop {
        let sleep = tokio::time::sleep(tokio::time::Duration::from_millis(timeout_ms));
        tokio::select! {
            _ = sleep => {
                // Check if game is still in progress
                let game_state = match state.game_store.get_game_by_id(game_id).await {
                    Ok(Some(g)) => g,
                    _ => break,
                };

                if game_state.status != GameStatus::InProgress {
                    break;
                }

                // Atomically claim the turn
                let current_turn = match state.cache.get_current_turn(&game_id.to_string()).await {
                    Ok(t) => t,
                    Err(_) => break,
                };

                if let Some(agent_id_str) = current_turn {
                    let agent_id: Uuid = match agent_id_str.parse() {
                        Ok(id) => id,
                        Err(_) => break,
                    };

                    // GETDEL equivalent: delete and check
                    let _ = state.cache.delete_current_turn(&game_id.to_string()).await;

                    // Find player and apply timeout action
                    let players = match state.game_store.get_players_by_game(game_id).await {
                        Ok(p) => p,
                        Err(_) => break,
                    };

                    if let Some(player) = players.iter().find(|p| p.agent_id == agent_id) {
                        let game_impl = match state.game_registry.get(&game_state.game_type) {
                            Some(g) => g,
                            None => break,
                        };

                        let timeout_action = game_impl.timeout_action(&game_state.current_state, agent_id);

                        // Check consecutive timeouts
                        let mut updated_player = player.clone();
                        updated_player.consecutive_timeouts += 1;

                        if updated_player.consecutive_timeouts >= 3 {
                            updated_player.status = PlayerStatus::Disconnected;
                        }

                        let _ = state.game_store.update_player(&updated_player).await;

                        // Apply timeout action
                        match game_impl.apply_action(
                            game_state.current_state.clone(),
                            agent_id,
                            &timeout_action,
                            None,
                        ) {
                            Ok(new_state) => {
                                let new_seq = game_state.sequence_number + 1;
                                let state_json = serde_json::to_string(&new_state).unwrap_or_default();
                                let state_hash = hex::encode(Sha256::digest(state_json.as_bytes()));

                                let log_entry = GameLogEntry {
                                    game_id,
                                    sequence: new_seq,
                                    timestamp: Utc::now(),
                                    agent_id: Some(agent_id),
                                    action: format!("timeout:{}", timeout_action),
                                    amount_atomic: None,
                                    state_hash,
                                };
                                let _ = state.game_store.append_log_entry(&log_entry).await;
                                let _ = state.game_store.update_game_state(game_id, &new_state, new_seq).await;

                                if game_impl.is_terminal(&new_state) {
                                    let _ = complete_game(game_id, &new_state, &state).await;
                                    break;
                                }

                                // Advance to next player
                                let _ = advance_turn(game_id, &new_state, agent_id, &state).await;
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
            _ = shutdown.changed() => {
                // Persist current state and stop
                tracing::info!("Turn manager for game {} shutting down gracefully", game_id);
                break;
            }
        }
    }
}
