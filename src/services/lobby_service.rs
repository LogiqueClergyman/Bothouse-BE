use chrono::Utc;
use sha3::{Digest, Keccak256};
use uuid::Uuid;

use crate::domain::game::{GameInstance, GamePlayer, GameStatus, PlayerStatus};
use crate::domain::lobby::{Room, RoomStatus, Seat};
use crate::errors::AppError;
use crate::state::AppState;

pub struct CreateRoomRequest {
    pub game_type: String,
    pub buy_in_wei: String,
    pub max_players: i16,
    pub min_players: i16,
    pub escrow_tx_hash: String,
}

pub struct JoinQueueRequest {
    pub game_type: String,
    pub buy_in_wei: String,
    pub max_players: i16,
    pub escrow_tx_hash: String,
}

pub struct JoinQueueResponse {
    pub room_id: Uuid,
    pub seat_number: i16,
    pub status: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RoomWithSeats {
    pub room: Room,
    pub seats: Vec<Seat>,
}

pub struct RoomFilters {
    pub game_type: Option<String>,
    pub status: Option<RoomStatus>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn create_room(
    agent_id: Uuid,
    req: CreateRoomRequest,
    state: &AppState,
) -> Result<RoomWithSeats, AppError> {
    let game = state
        .game_registry
        .get(&req.game_type)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown game type: {}", req.game_type)))?;

    if req.max_players < game.min_players() as i16 || req.max_players > game.max_players() as i16 {
        return Err(AppError::BadRequest("max_players out of range".to_string()));
    }
    if req.min_players < 2 || req.min_players > req.max_players {
        return Err(AppError::BadRequest("min_players invalid".to_string()));
    }

    let buy_in: u128 = req
        .buy_in_wei
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid buy_in_wei".to_string()))?;
    if buy_in == 0 {
        return Err(AppError::BadRequest("buy_in_wei must be > 0".to_string()));
    }

    let agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let now = Utc::now();
    let room = Room {
        room_id: Uuid::new_v4(),
        game_type: req.game_type.clone(),
        game_version: game.version().to_string(),
        status: RoomStatus::Open,
        buy_in_wei: req.buy_in_wei.clone(),
        max_players: req.max_players,
        min_players: req.min_players,
        created_at: now,
        started_at: None,
        completed_at: None,
    };

    let room = state.lobby_store.create_room(&room).await?;

    // Verify escrow for creator
    let verified = state
        .settlement
        .check_escrow_deposit(room.room_id, &agent.wallet_address, &req.buy_in_wei)
        .await?;
    if !verified {
        return Err(AppError::BadRequest("ESCROW_NOT_VERIFIED".to_string()));
    }

    let seat = Seat {
        seat_id: Uuid::new_v4(),
        room_id: room.room_id,
        agent_id,
        wallet_address: agent.wallet_address.clone(),
        seat_number: 1,
        joined_at: now,
        escrow_tx_hash: Some(req.escrow_tx_hash.clone()),
        escrow_verified: true,
    };
    let seat = state.lobby_store.create_seat(&seat).await?;

    Ok(RoomWithSeats {
        room,
        seats: vec![seat],
    })
}

pub async fn list_rooms(
    filters: RoomFilters,
    state: &AppState,
) -> Result<Vec<RoomWithSeats>, AppError> {
    let rooms = state
        .lobby_store
        .list_rooms(filters.status, filters.game_type.as_deref(), filters.limit, filters.offset)
        .await?;

    let mut result = Vec::new();
    for room in rooms {
        let seats = state.lobby_store.get_seats_by_room(room.room_id).await?;
        result.push(RoomWithSeats { room, seats });
    }
    Ok(result)
}

pub async fn get_room(room_id: Uuid, state: &AppState) -> Result<RoomWithSeats, AppError> {
    let room = state
        .lobby_store
        .get_room_by_id(room_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let seats = state.lobby_store.get_seats_by_room(room_id).await?;
    Ok(RoomWithSeats { room, seats })
}

pub async fn join_room(
    agent_id: Uuid,
    room_id: Uuid,
    escrow_tx_hash: &str,
    state: &AppState,
) -> Result<Seat, AppError> {
    let room = state
        .lobby_store
        .get_room_by_id(room_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if room.status != RoomStatus::Open {
        return Err(AppError::Conflict("ROOM_NOT_OPEN".to_string()));
    }

    let existing = state
        .lobby_store
        .get_seat_by_agent_and_room(agent_id, room_id)
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("Agent already seated".to_string()));
    }

    let seats = state.lobby_store.get_seats_by_room(room_id).await?;
    if seats.len() >= room.max_players as usize {
        return Err(AppError::Conflict("ROOM_FULL".to_string()));
    }

    let agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let verified = state
        .settlement
        .check_escrow_deposit(room_id, &agent.wallet_address, &room.buy_in_wei)
        .await?;
    if !verified {
        return Err(AppError::BadRequest("ESCROW_NOT_VERIFIED".to_string()));
    }

    let seat_number = seats.len() as i16 + 1;
    let seat = Seat {
        seat_id: Uuid::new_v4(),
        room_id,
        agent_id,
        wallet_address: agent.wallet_address.clone(),
        seat_number,
        joined_at: Utc::now(),
        escrow_tx_hash: Some(escrow_tx_hash.to_string()),
        escrow_verified: true,
    };
    let seat = state.lobby_store.create_seat(&seat).await?;

    // Check if we should start
    let updated_seats = state.lobby_store.get_seats_by_room(room_id).await?;
    let all_verified = updated_seats.iter().all(|s| s.escrow_verified);
    if updated_seats.len() >= room.min_players as usize && all_verified {
        let _ = start_game(room_id, state).await;
    }

    Ok(seat)
}

pub async fn leave_room(
    agent_id: Uuid,
    room_id: Uuid,
    state: &AppState,
) -> Result<(), AppError> {
    let room = state
        .lobby_store
        .get_room_by_id(room_id)
        .await?
        .ok_or(AppError::NotFound)?;

    match room.status {
        RoomStatus::InProgress | RoomStatus::Completed | RoomStatus::Starting => {
            return Err(AppError::Forbidden("GAME_ALREADY_STARTED".to_string()));
        }
        _ => {}
    }

    let seat = state
        .lobby_store
        .get_seat_by_agent_and_room(agent_id, room_id)
        .await?
        .ok_or(AppError::NotFound)?;

    state.lobby_store.delete_seat(seat.seat_id).await?;
    Ok(())
}

pub async fn join_queue(
    agent_id: Uuid,
    req: JoinQueueRequest,
    state: &AppState,
) -> Result<JoinQueueResponse, AppError> {
    // Look for existing open room matching game_type, buy_in_wei, max_players
    let rooms = state
        .lobby_store
        .list_rooms(
            Some(RoomStatus::Open),
            Some(&req.game_type),
            100,
            0,
        )
        .await?;

    for room in rooms {
        if room.buy_in_wei == req.buy_in_wei && room.max_players == req.max_players {
            let seats = state.lobby_store.get_seats_by_room(room.room_id).await?;
            if seats.len() < room.max_players as usize {
                let seat = join_room(agent_id, room.room_id, &req.escrow_tx_hash, state).await?;
                return Ok(JoinQueueResponse {
                    room_id: room.room_id,
                    seat_number: seat.seat_number,
                    status: "seated".to_string(),
                });
            }
        }
    }

    // Create new room
    let room_req = CreateRoomRequest {
        game_type: req.game_type.clone(),
        buy_in_wei: req.buy_in_wei.clone(),
        max_players: req.max_players,
        min_players: 2,
        escrow_tx_hash: req.escrow_tx_hash,
    };
    let room_with_seats = create_room(agent_id, room_req, state).await?;
    let seat_number = room_with_seats.seats[0].seat_number;
    Ok(JoinQueueResponse {
        room_id: room_with_seats.room.room_id,
        seat_number,
        status: "seated".to_string(),
    })
}

pub async fn start_game(room_id: Uuid, state: &AppState) -> Result<GameInstance, AppError> {
    state
        .lobby_store
        .update_room_status(room_id, RoomStatus::Starting)
        .await?;

    let room = state
        .lobby_store
        .get_room_by_id(room_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let seats = state.lobby_store.get_seats_by_room(room_id).await?;

    // Generate seed: keccak256(room_id_bytes ++ all agent_wallet_bytes ++ house_secret_bytes)
    let mut hasher = Keccak256::new();
    hasher.update(room_id.as_bytes());
    for seat in &seats {
        hasher.update(seat.wallet_address.as_bytes());
    }
    let house_secret_bytes = hex::decode(&state.config.house_signing_key)
        .unwrap_or_else(|_| state.config.house_signing_key.as_bytes().to_vec());
    hasher.update(&house_secret_bytes);
    let seed_bytes = hasher.finalize();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_bytes);

    let game_impl = state
        .game_registry
        .get(&room.game_type)
        .ok_or_else(|| AppError::BadRequest("Unknown game type".to_string()))?;

    let players: Vec<GamePlayer> = seats
        .iter()
        .map(|s| GamePlayer {
            game_id: Uuid::nil(),
            agent_id: s.agent_id,
            wallet_address: s.wallet_address.clone(),
            seat_number: s.seat_number,
            stack_wei: room.buy_in_wei.clone(),
            status: PlayerStatus::Active,
            consecutive_timeouts: 0,
        })
        .collect();

    let initial_state = game_impl.init(players.clone(), seed)?;

    let now = Utc::now();
    let game_id = Uuid::new_v4();

    let game_instance = GameInstance {
        game_id,
        room_id,
        game_type: room.game_type.clone(),
        game_version: room.game_version.clone(),
        status: GameStatus::InProgress,
        current_state: initial_state,
        sequence_number: 0,
        created_at: now,
        started_at: Some(now),
        completed_at: None,
    };

    let game_instance = state.game_store.create_game(&game_instance).await?;

    for mut p in players {
        p.game_id = game_id;
        state.game_store.create_player(&p).await?;
    }

    state
        .lobby_store
        .update_room_status(room_id, RoomStatus::InProgress)
        .await?;

    // Spawn turn manager
    let state_clone = state.clone();
    let game_id_clone = game_id;
    let shutdown_rx = state.shutdown_tx.subscribe();
    tokio::spawn(async move {
        crate::services::game_service::run_turn_manager(game_id_clone, state_clone, shutdown_rx)
            .await;
    });

    Ok(game_instance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::*;
    use crate::config::Config;
    use crate::games::GameRegistry;
    use std::sync::Arc;
    use tokio::sync::watch;

    fn make_state() -> AppState {
        let config = Config {
            database_url: "".to_string(),
            redis_url: "".to_string(),
            jwt_secret: "test".to_string(),
            jwt_expiry_secs: 86400,
            refresh_token_expiry_secs: 2592000,
            bcrypt_cost: 4,
            house_signing_key: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
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

        let mut registry = GameRegistry::new();
        registry.register(Box::new(crate::games::texas_holdem_v1::engine::TexasHoldemV1));

        let (shutdown_tx, _) = watch::channel(());
        AppState {
            auth_store: Arc::new(auth_store::MemoryAuthStore::new()),
            agent_store: Arc::new(agent_store::MemoryAgentStore::new()),
            game_store: Arc::new(game_store::MemoryGameStore::new()),
            lobby_store: Arc::new(lobby_store::MemoryLobbyStore::new()),
            cache: Arc::new(cache_store::MemoryCacheStore::new()),
            event_bus: Arc::new(event_bus::MemoryEventBus::new()),
            settlement: Arc::new(NoopSettlement),
            http_client: Arc::new(NoopHttpClient),
            game_registry: Arc::new(registry),
            config: Arc::new(config),
            shutdown_tx: Arc::new(shutdown_tx),
        }
    }

    async fn register_test_agent(state: &AppState) -> (Uuid, Uuid) {
        let user_id = Uuid::new_v4();
        let agent = crate::domain::agent::Agent {
            agent_id: Uuid::new_v4(),
            user_id,
            wallet_address: format!("0x{:0<40}", hex::encode(Uuid::new_v4().as_bytes())),
            name: "TestAgent".to_string(),
            description: None,
            webhook_url: None,
            status: crate::domain::agent::AgentStatus::Active,
            api_key_hash: "hash".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_seen_at: None,
        };
        state.agent_store.create_agent(&agent).await.unwrap();
        (user_id, agent.agent_id)
    }

    #[tokio::test]
    async fn test_create_room() {
        let state = make_state();
        let (_, agent_id) = register_test_agent(&state).await;
        let req = CreateRoomRequest {
            game_type: "texas_holdem_v1".to_string(),
            buy_in_wei: "1000000000000000000".to_string(),
            max_players: 6,
            min_players: 2,
            escrow_tx_hash: "0xabc".to_string(),
        };
        let result = create_room(agent_id, req, &state).await.unwrap();
        assert_eq!(result.seats.len(), 1);
        assert_eq!(result.room.status, RoomStatus::Open);
    }

    #[tokio::test]
    async fn test_join_room() {
        let state = make_state();
        let (_, creator_id) = register_test_agent(&state).await;
        let (_, joiner_id) = register_test_agent(&state).await;

        let req = CreateRoomRequest {
            game_type: "texas_holdem_v1".to_string(),
            buy_in_wei: "1000000000000000000".to_string(),
            max_players: 6,
            min_players: 3,
            escrow_tx_hash: "0xabc".to_string(),
        };
        let room = create_room(creator_id, req, &state).await.unwrap();
        let seat = join_room(joiner_id, room.room.room_id, "0xdef", &state).await.unwrap();
        assert_eq!(seat.seat_number, 2);
    }
}
