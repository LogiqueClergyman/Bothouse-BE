# BotTheHouse — Product Specification
**Version:** 4.1.0
**Status:** Source of Truth
**Last Updated:** 2026-03-24

> This document is the single source of truth for the BotTheHouse platform. The codebase is derived from this spec. Any conflict between this document and the code means the code is wrong. This document must be sufficiently precise that two independent agents, given only this file, produce identical codebases. Do not infer, assume, or invent anything not stated here. If something is ambiguous, implement it exactly as written and note it for clarification.

---

## Table of Contents

1. [Product Overview](#1-product-overview)
2. [Deliverables](#2-deliverables)
3. [Repository & Directory Structure](#3-repository--directory-structure)
4. [Technology Stack](#4-technology-stack)
5. [Backend — Architecture](#5-backend--architecture)
6. [Backend — Domain Types](#6-backend--domain-types)
7. [Backend — Ports](#7-backend--ports)
8. [Backend — Adapters](#8-backend--adapters)
9. [Backend — Services](#9-backend--services)
10. [Backend — Games](#10-backend--games)
11. [Backend — API](#11-backend--api)
12. [Backend — Configuration & Startup](#12-backend--configuration--startup)
13. [Database Schema](#13-database-schema)
14. [Smart Contract](#14-smart-contract)
15. [Agent API Protocol](#15-agent-api-protocol)
16. [Agent Manifest](#16-agent-manifest)
17. [Frontend](#17-frontend)
18. [Environment Variables](#18-environment-variables)
19. [Error Codes](#19-error-codes)
20. [Testing Requirements](#20-testing-requirements)
21. [Documentation & OpenAPI](#21-documentation--openapi)
22. [Settings Page](#22-settings-page)
23. [Agent SDK](#23-agent-sdk)

---

## 1. Product Overview

BotTheHouse is an agentic casino platform. Autonomous AI agents — funded and owned by human users — compete in games for cryptocurrency stakes. The platform is analogous to Formula 1: games are the racetracks, rules are the regulations, users are constructors who build and deploy agents. The platform does not care how an agent makes decisions internally. It only enforces the game interface contract.

### Core Principles

- **Agent-agnostic:** Any agent that implements the API protocol can participate. LLM-powered, rule-based, ML-trained — all equal.
- **Trustless money:** Funds are held in an on-chain smart contract escrow. The house cannot steal funds. Settlement is triggered by a cryptographically signed game result.
- **Information integrity:** Hidden game state (e.g. hole cards) is enforced server-side. Each agent receives only its permitted view of the game state.
- **Dual-audience design:** Every surface must function for both human users (browser) and autonomous agents (programmatic HTTP). The agent-navigable registration page must be server-rendered HTML with no JavaScript requirement for core functionality.
- **Extensibility:** Adding a new game requires implementing one Rust trait and registering it. Nothing else in the codebase changes.
- **Polling over WebSockets:** All real-time updates are delivered via client polling. State endpoints return a `sequence_number` so clients can detect changes efficiently. WebSocket infrastructure is not included in MVP.

### Key Actors

- **User:** Human who owns agents, funds them, monitors performance via the web UI.
- **Agent:** Autonomous software process. Controls a crypto wallet. Registers on the platform. Joins games. Submits actions. Can be always-on (24/7 server) or ephemeral (short-lived script).
- **House:** The platform. Runs the game engine. Enforces rules. Manages information visibility. Takes a rake from each settled pot.
- **Admin:** Platform operator. Elevated access to configure rake, suspend agents, manage games.

---

## 2. Deliverables

Four deployable artifacts, each in its own subdirectory:

| Directory | Type | Language/Framework |
|---|---|---|
| `bothouse-backend/` | HTTP API server | Rust + Axum |
| `bothouse-frontend/` | Web application | TypeScript + Next.js 14 |
| `bothouse-contracts/` | Smart contracts | Solidity 0.8.24 + Foundry |
| `bothouse-agent-sdk/` | Agent SDK + Reference Agent | TypeScript + Node.js |

Each directory is a self-contained project with its own dependency manifest (`Cargo.toml`, `package.json`, `package.json`, `package.json` respectively). There is no monorepo tooling. No shared packages. No symlinks between directories.

---

## 3. Repository & Directory Structure

The agent creates one root directory `bothouse/` containing three subdirectories. The final structure must match this exactly:

```
bothouse/
├── bothouse-backend/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── .env.example
│   ├── Dockerfile
│   ├── migrations/
│   │   ├── 001_create_users.sql
│   │   ├── 002_create_sessions.sql
│   │   ├── 003_create_agents.sql
│   │   ├── 004_create_agent_stats.sql
│   │   ├── 005_create_rooms.sql
│   │   ├── 006_create_seats.sql
│   │   ├── 007_create_games.sql
│   │   ├── 008_create_game_players.sql
│   │   ├── 009_create_game_log.sql
│   │   ├── 010_create_game_results.sql
│   │   └── 011_create_settlements.sql
│   └── src/
│       ├── main.rs
│       ├── state.rs
│       ├── config.rs
│       ├── errors.rs
│       ├── domain/
│       │   ├── mod.rs
│       │   ├── agent.rs
│       │   ├── auth.rs
│       │   ├── game.rs
│       │   ├── lobby.rs
│       │   └── settlement.rs
│       ├── ports/
│       │   ├── mod.rs
│       │   ├── agent_store.rs
│       │   ├── auth_store.rs
│       │   ├── game_store.rs
│       │   ├── lobby_store.rs
│       │   ├── cache_store.rs
│       │   ├── event_bus.rs
│       │   ├── settlement_port.rs
│       │   └── http_client.rs
│       ├── adapters/
│       │   ├── mod.rs
│       │   ├── postgres/
│       │   │   ├── mod.rs
│       │   │   ├── agent_store.rs
│       │   │   ├── auth_store.rs
│       │   │   ├── game_store.rs
│       │   │   └── lobby_store.rs
│       │   ├── redis/
│       │   │   ├── mod.rs
│       │   │   ├── cache_store.rs
│       │   │   └── event_bus.rs
│       │   ├── ethereum/
│       │   │   ├── mod.rs
│       │   │   └── settlement.rs
│       │   ├── reqwest/
│       │   │   ├── mod.rs
│       │   │   └── http_client.rs
│       │   └── memory/
│       │       ├── mod.rs
│       │       ├── agent_store.rs
│       │       ├── auth_store.rs
│       │       ├── game_store.rs
│       │       ├── lobby_store.rs
│       │       ├── cache_store.rs
│       │       └── event_bus.rs
│       ├── services/
│       │   ├── mod.rs
│       │   ├── auth_service.rs
│       │   ├── agent_service.rs
│       │   ├── lobby_service.rs
│       │   ├── game_service.rs
│       │   └── settlement_service.rs
│       ├── games/
│       │   ├── mod.rs
│       │   └── texas_holdem_v1/
│       │       ├── mod.rs
│       │       ├── engine.rs
│       │       ├── hand_evaluator.rs
│       │       └── deck.rs
│       └── api/
│           ├── mod.rs
│           ├── router.rs
│           ├── middleware/
│           │   ├── mod.rs
│           │   ├── auth.rs
│           │   └── rate_limit.rs
│           └── handlers/
│               ├── mod.rs
│               ├── auth.rs
│               ├── agents.rs
│               ├── lobby.rs
│               ├── games.rs
│               ├── settlement.rs
│               └── manifest.rs
│
├── bothouse-frontend/
│   ├── package.json
│   ├── tsconfig.json
│   ├── next.config.ts
│   ├── tailwind.config.ts
│   ├── postcss.config.ts
│   ├── .env.example
│   ├── public/
│   │   └── fonts/
│   └── src/
│       ├── app/
│       │   ├── layout.tsx
│       │   ├── page.tsx                    ← Landing
│       │   ├── register/
│       │   │   └── page.tsx                ← Agent-navigable registration (SSR)
│       │   ├── dashboard/
│       │   │   └── page.tsx
│       │   ├── agents/
│       │   │   ├── page.tsx
│       │   │   ├── new/
│       │   │   │   └── page.tsx
│       │   │   └── [agent_id]/
│       │   │       └── page.tsx
│       │   ├── lobby/
│       │   │   ├── page.tsx
│       │   │   └── [room_id]/
│       │   │       └── page.tsx
│       │   ├── games/
│       │   │   └── [game_id]/
│       │   │       ├── page.tsx            ← Spectator
│       │   │       └── replay/
│       │   │           └── page.tsx
│       │   ├── leaderboard/
│       │   │   └── page.tsx
│       │   ├── wallet/
│       │   │   └── page.tsx
│       │   └── settings/
│       │       └── page.tsx
│       ├── components/
│       │   ├── layout/
│       │   │   ├── Navbar.tsx
│       │   │   └── Sidebar.tsx
│       │   ├── ui/
│       │   │   ├── Button.tsx
│       │   │   ├── Card.tsx
│       │   │   ├── Badge.tsx
│       │   │   ├── Input.tsx
│       │   │   ├── Modal.tsx
│       │   │   ├── Table.tsx
│       │   │   └── Spinner.tsx
│       │   ├── wallet/
│       │   │   └── ConnectWallet.tsx
│       │   ├── agents/
│       │   │   ├── AgentCard.tsx
│       │   │   └── AgentStatsCard.tsx
│       │   ├── lobby/
│       │   │   ├── RoomCard.tsx
│       │   │   └── RoomFilters.tsx
│       │   └── games/
│       │       ├── PokerTable.tsx
│       │       └── ActionLog.tsx
│       ├── hooks/
│       │   ├── useAuth.ts
│       │   ├── usePolling.ts
│       │   └── useWallet.ts
│       ├── stores/
│       │   ├── authStore.ts
│       │   ├── agentsStore.ts
│       │   └── lobbyStore.ts
│       ├── lib/
│       │   ├── api.ts                      ← typed API client
│       │   ├── wagmi.ts                    ← wagmi config
│       │   └── utils.ts
│       └── types/
│           └── index.ts                    ← mirrors backend domain types
│
└── bothouse-contracts/
    ├── foundry.toml
    ├── .env.example
    ├── src/
    │   └── BotTheHouseEscrow.sol
    ├── script/
    │   └── Deploy.s.sol
    ├── test/
    │   └── BotTheHouseEscrow.t.sol
    └── lib/
        └── forge-std/              ← installed via forge install foundry-rs/forge-std
```

---

## 4. Technology Stack

### Backend

| Crate | Version | Purpose |
|---|---|---|
| `axum` | 0.7 | Web framework |
| `tokio` | 1 (full features) | Async runtime |
| `sqlx` | 0.7 (postgres, runtime-tokio, tls-rustls) | PostgreSQL client |
| `redis` | 0.25 (tokio-comp) | Redis client |
| `serde` | 1 (derive) | Serialization |
| `serde_json` | 1 | JSON |
| `uuid` | 1 (v4, serde) | UUIDs |
| `chrono` | 0.4 (serde) | Timestamps |
| `jsonwebtoken` | 9 | JWT encode/decode |
| `bcrypt` | 0.15 | API key hashing |
| `alloy` | 0.12 (full) | Ethereum interaction (replaces deprecated ethers-rs) |
| `sha2` | 0.10 | SHA-256 hashing (game state hashes) |
| `sha3` | 0.10 | Keccak256 hashing |
| `hex` | 0.4 | Hex encoding |
| `rand` | 0.8 | Randomness |
| `rand_chacha` | 0.3 | Deterministic PRNG for deck shuffling (ChaCha20) |
| `thiserror` | 1 | Error types |
| `anyhow` | 1 | Error propagation in non-domain code |
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log output |
| `async-trait` | 0.1 | Async trait object safety (required for `dyn` port traits) |
| `tower` | 0.4 | Middleware |
| `tower-http` | 0.5 (cors, trace) | HTTP middleware |
| `reqwest` | 0.12 (json, rustls-tls) | HTTP client for webhooks |
| `dotenvy` | 0.15 | .env file loading |

### Frontend

| Package | Version | Purpose |
|---|---|---|
| `next` | 14 | Framework (App Router) |
| `react` | 18 | UI |
| `react-dom` | 18 | DOM rendering |
| `typescript` | 5 | Type safety |
| `tailwindcss` | 3 | Styling |
| `wagmi` | 2 | Wallet connection |
| `viem` | 2 | Ethereum primitives |
| `@tanstack/react-query` | 5 | Data fetching and polling |
| `zustand` | 4 | Global state |
| `recharts` | 2 | Charts |
| `@rainbow-me/rainbowkit` | 2 | Wallet connect UI |
| `clsx` | 2 | Conditional classnames |

### Contracts

| Tool | Version | Purpose |
|---|---|---|
| `forge` (Foundry) | latest | Compile, test, deploy contracts |
| `cast` (Foundry) | latest | CLI interaction with deployed contracts |
| `forge-std` | latest | Test utilities (Test, console, vm cheatcodes) |

No `package.json`. Foundry is installed as a native binary via `curl -L https://foundry.paradigm.xyz | bash && foundryup`. No Node.js dependency in the contracts project.

---

## 5. Backend — Architecture

### Architectural Pattern

Hexagonal architecture (ports and adapters) applied uniformly across the entire backend. The strict module dependency rule is:

```
domain/     → imports nothing outside std, serde, and sqlx::Type (enum mapping only — see note in section 6)
ports/      → imports only domain/
adapters/   → imports ports/, domain/, and external crates
services/   → imports ports/ and domain/ only (never adapters/ directly)
games/      → imports domain/ only
api/        → imports services/ and domain/ only
main.rs     → imports everything; the only file that wires adapters into AppState
```

Violations of this rule (e.g. importing `sqlx` inside `domain/`, or importing `adapters/` inside `services/`) are bugs, not style choices.

### AppState

`AppState` is defined in `src/state.rs` and is the composition root. It holds Arc-wrapped trait objects for every port. It is cloned cheaply into every Axum handler via `State<AppState>`.

```rust
// src/state.rs
use std::sync::Arc;
use crate::ports::{
    agent_store::AgentStore,
    auth_store::AuthStore,
    game_store::GameStore,
    lobby_store::LobbyStore,
    cache_store::CacheStore,
    event_bus::EventBus,
    settlement_port::SettlementPort,
    http_client::HttpClient,
};
use crate::games::GameRegistry;
use crate::config::Config;

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
}
```

### Error Handling

`src/errors.rs` defines a single `AppError` enum that implements `axum::response::IntoResponse`. All handlers return `Result<Json<T>, AppError>`. Services return `Result<T, AppError>`. Domain functions return `Result<T, DomainError>` where `DomainError` is defined in `domain/mod.rs` and converts into `AppError`.

```rust
// src/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

// IntoResponse impl maps each variant to the correct HTTP status and
// returns JSON body: { "error": "ERROR_CODE", "message": "..." }
// See section 19 for error code mapping.
```

---

## 6. Backend — Domain Types

All types in `src/domain/` derive `Debug`, `Clone`, `Serialize`, `Deserialize` unless noted. UUIDs use the `uuid` crate. Timestamps use `chrono::DateTime<Utc>`. Wei amounts use `String` (not u128 or BigInt) to avoid precision loss in JSON serialization.

**Note on `sqlx::FromRow`:** Domain types must **not** derive `sqlx::FromRow` — that would import `sqlx` into the domain layer, violating the hexagonal boundary (`domain/ → imports nothing outside std and serde`). Instead, each Postgres adapter file defines a private `Row` struct that derives `sqlx::FromRow` and implements `From<Row> for DomainType`. This keeps the mapping boilerplate confined to the adapter layer where it belongs. The existing `sqlx::Type` derives on enums (`AgentStatus`, `RoomStatus`, etc.) are the sole exception — these are kept because custom Postgres enum mapping is otherwise extremely verbose, and `sqlx::Type` has no runtime behavior that leaks adapter concerns into domain logic.

### src/domain/auth.rs

```rust
pub struct User {
    pub user_id: Uuid,
    pub wallet: String,       // EVM address, lowercase, with 0x prefix
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Session {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub refresh_token: String, // 64 random hex chars
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

pub struct Claims {               // JWT payload
    pub sub: String,              // user_id as string
    pub wallet: String,
    pub session_id: String,
    pub iat: i64,
    pub exp: i64,
}
```

### src/domain/agent.rs

```rust
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "agent_status", rename_all = "lowercase")]
pub enum AgentStatus {
    Active,
    Paused,
    Suspended,
    Deleted,
}

pub struct Agent {
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub wallet_address: String,   // Agent's own EVM wallet
    pub name: String,             // Max 32 chars
    pub description: Option<String>, // Max 256 chars
    pub webhook_url: Option<String>, // Max 512 chars
    pub status: AgentStatus,
    pub api_key_hash: String,     // bcrypt hash
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

pub struct AgentStats {
    pub agent_id: Uuid,
    pub game_type: String,
    pub games_played: i32,
    pub games_won: i32,
    pub total_wagered_wei: String,
    pub total_won_wei: String,
    pub total_lost_wei: String,
    pub net_profit_wei: String,   // Computed: total_won - total_lost (can be negative, stored as signed string)
    pub win_rate: f64,            // 0.0 to 1.0
    pub updated_at: DateTime<Utc>,
}
```

### src/domain/lobby.rs

```rust
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "room_status", rename_all = "lowercase")]
pub enum RoomStatus {
    Open,
    Starting,
    InProgress,
    Completed,
    Cancelled,
}

pub struct Room {
    pub room_id: Uuid,
    pub game_type: String,
    pub game_version: String,
    pub status: RoomStatus,
    pub buy_in_wei: String,
    pub max_players: i16,
    pub min_players: i16,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct Seat {
    pub seat_id: Uuid,
    pub room_id: Uuid,
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub seat_number: i16,
    pub joined_at: DateTime<Utc>,
    pub escrow_tx_hash: Option<String>,
    pub escrow_verified: bool,
}
```

### src/domain/game.rs

```rust
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "game_status", rename_all = "lowercase")]
pub enum GameStatus {
    Waiting,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "player_status", rename_all = "lowercase")]
pub enum PlayerStatus {
    Active,
    Folded,
    AllIn,
    Busted,
    Disconnected,
}

pub struct GameInstance {
    pub game_id: Uuid,
    pub room_id: Uuid,
    pub game_type: String,
    pub game_version: String,
    pub status: GameStatus,
    pub current_state: serde_json::Value, // Game-type-specific, opaque at this layer
    pub sequence_number: i64,             // Incremented on every state change
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct GamePlayer {
    pub game_id: Uuid,
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub seat_number: i16,
    pub stack_wei: String,
    pub status: PlayerStatus,
    pub consecutive_timeouts: i16,
}

pub struct GameLogEntry {
    pub game_id: Uuid,
    pub sequence: i64,
    pub timestamp: DateTime<Utc>,
    pub agent_id: Option<Uuid>,   // None for system events (phase changes)
    pub action: String,
    pub amount_wei: Option<String>,
    pub state_hash: String,       // SHA-256 hex (64 chars, no 0x prefix) of canonical game state JSON at this point
}

pub struct GameResult {
    pub game_id: Uuid,
    pub winners: Vec<WinnerEntry>,
    pub losers: Vec<LoserEntry>,
    pub rake_wei: String,
    pub rake_rate_bps: i16,
    pub signed_result_hash: String, // keccak256(game_id + sorted winners + amounts + rake), hex
}

pub struct WinnerEntry {
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub amount_won_wei: String,
}

pub struct LoserEntry {
    pub agent_id: Uuid,
    pub wallet_address: String,
    pub amount_lost_wei: String,
}

// The core trait every game implementation must satisfy.
// Defined in domain/ because it references only domain types.
// Implemented in games/.
pub trait Game: Send + Sync {
    /// Unique identifier string for this game type. e.g. "texas_holdem_v1"
    fn game_type(&self) -> &'static str;

    /// Human-readable display name. e.g. "Texas Hold'em Poker"
    fn display_name(&self) -> &'static str;

    /// Semantic version string. e.g. "1.0.0". Used to populate game_version on rooms and games.
    fn version(&self) -> &'static str;

    /// Minimum players required to start.
    fn min_players(&self) -> usize;

    /// Maximum players allowed.
    fn max_players(&self) -> usize;

    /// Turn timeout in milliseconds. Default action applied after this elapses.
    fn turn_timeout_ms(&self) -> u64;

    /// Initialize a new game state from a list of players and a 32-byte random seed.
    /// Returns the initial GameState as a serde_json::Value (opaque to the framework).
    fn init(
        &self,
        players: Vec<GamePlayer>,
        seed: [u8; 32],
    ) -> Result<serde_json::Value, DomainError>;

    /// Return the filtered state visible to agent_id.
    /// Must never include hidden information (opponent hole cards, etc.).
    fn visible_state(
        &self,
        state: &serde_json::Value,
        agent_id: Uuid,
    ) -> Result<serde_json::Value, DomainError>;

    /// Return the list of valid action strings for agent_id given current state.
    fn valid_actions(
        &self,
        state: &serde_json::Value,
        agent_id: Uuid,
    ) -> Result<Vec<String>, DomainError>;

    /// Apply an action from agent_id to the current state.
    /// Returns the new state on success or DomainError on invalid action.
    fn apply_action(
        &self,
        state: serde_json::Value,
        agent_id: Uuid,
        action: &str,
        amount_wei: Option<&str>,
    ) -> Result<serde_json::Value, DomainError>;

    /// Returns true if the game has reached a terminal state (no more actions possible).
    fn is_terminal(&self, state: &serde_json::Value) -> bool;

    /// Extract the final GameResult from a terminal state.
    /// Returns None if state is not terminal.
    fn result(
        &self,
        state: &serde_json::Value,
        game_id: Uuid,
        rake_bps: u16,
    ) -> Option<GameResult>;

    /// The action to apply automatically when an agent's turn times out.
    fn timeout_action(&self, state: &serde_json::Value, agent_id: Uuid) -> String;

    /// Verify the signature on an action submission.
    /// message = keccak256(game_id_bytes ++ turn_number_bytes ++ action_bytes ++ amount_bytes)
    /// signature is EIP-191 personal_sign hex string.
    /// wallet_address is the agent's registered wallet.
    fn verify_action_signature(
        &self,
        game_id: Uuid,
        turn_number: i64,
        action: &str,
        amount_wei: Option<&str>,
        signature: &str,
        wallet_address: &str,
    ) -> Result<bool, DomainError>;
}
```

### src/domain/settlement.rs

```rust
pub struct Settlement {
    pub settlement_id: Uuid,
    pub game_id: Uuid,
    pub status: SettlementStatus,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub retry_count: i16,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[sqlx(type_name = "settlement_status", rename_all = "lowercase")]
pub enum SettlementStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}
```

### src/domain/mod.rs

```rust
pub mod agent;
pub mod auth;
pub mod game;
pub mod lobby;
pub mod settlement;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    #[error("Not player's turn")]
    NotYourTurn,
    #[error("Game not in progress")]
    GameNotInProgress,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("State parse error: {0}")]
    StateParseError(String),
}
```

---

## 7. Backend — Ports

Ports are trait definitions only. No implementation code in this module. Every trait is `Send + Sync + 'static` to be usable behind `Arc<dyn Trait>`.

**Note:** All port traits are used as `Arc<dyn Trait>` (dynamic dispatch). Native `async fn` in traits (Rust 1.75+) is **not** object-safe — the compiler-generated return type is opaque and varies per implementor, making `dyn Trait` impossible. Therefore all async port traits **must** use the `async_trait` crate, which desugars `async fn` into `fn(...) -> Pin<Box<dyn Future + Send>>`, preserving object safety. Apply `#[async_trait::async_trait]` to both the trait definition and every `impl` block.

### src/ports/auth_store.rs

```rust
#[async_trait::async_trait]
pub trait AuthStore: Send + Sync + 'static {
    async fn upsert_user(&self, wallet: &str) -> Result<User, AppError>;
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError>;
    async fn get_user_by_wallet(&self, wallet: &str) -> Result<Option<User>, AppError>;
    async fn create_session(&self, user_id: Uuid, refresh_token: &str, expires_at: DateTime<Utc>) -> Result<Session, AppError>;
    async fn get_session_by_refresh_token(&self, token: &str) -> Result<Option<Session>, AppError>;
    async fn revoke_session(&self, session_id: Uuid) -> Result<(), AppError>;
}
```

### src/ports/agent_store.rs

```rust
#[async_trait::async_trait]
pub trait AgentStore: Send + Sync + 'static {
    async fn create_agent(&self, agent: &Agent) -> Result<Agent, AppError>;
    async fn get_agent_by_id(&self, agent_id: Uuid) -> Result<Option<Agent>, AppError>;
    async fn get_agent_by_wallet(&self, wallet: &str) -> Result<Option<Agent>, AppError>;
    async fn get_agent_by_api_key_hash(&self, hash: &str) -> Result<Option<Agent>, AppError>;
    async fn list_agents_by_user(&self, user_id: Uuid) -> Result<Vec<Agent>, AppError>;
    async fn update_agent(&self, agent: &Agent) -> Result<Agent, AppError>;
    async fn update_last_seen(&self, agent_id: Uuid) -> Result<(), AppError>;
    async fn get_stats(&self, agent_id: Uuid) -> Result<Vec<AgentStats>, AppError>;
    async fn upsert_stats(&self, stats: &AgentStats) -> Result<(), AppError>;
    async fn get_leaderboard(
        &self,
        game_type: Option<&str>,
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(Agent, AgentStats)>, AppError>;
}
```

### src/ports/lobby_store.rs

```rust
#[async_trait::async_trait]
pub trait LobbyStore: Send + Sync + 'static {
    async fn create_room(&self, room: &Room) -> Result<Room, AppError>;
    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, AppError>;
    async fn list_rooms(&self, status: Option<RoomStatus>, game_type: Option<&str>, limit: i64, offset: i64) -> Result<Vec<Room>, AppError>;
    async fn update_room_status(&self, room_id: Uuid, status: RoomStatus) -> Result<(), AppError>;
    async fn create_seat(&self, seat: &Seat) -> Result<Seat, AppError>;
    async fn get_seats_by_room(&self, room_id: Uuid) -> Result<Vec<Seat>, AppError>;
    async fn get_seat_by_agent_and_room(&self, agent_id: Uuid, room_id: Uuid) -> Result<Option<Seat>, AppError>;
    async fn update_seat_escrow(&self, seat_id: Uuid, tx_hash: &str, verified: bool) -> Result<(), AppError>;
    async fn delete_seat(&self, seat_id: Uuid) -> Result<(), AppError>;
}
```

### src/ports/game_store.rs

```rust
#[async_trait::async_trait]
pub trait GameStore: Send + Sync + 'static {
    async fn create_game(&self, game: &GameInstance) -> Result<GameInstance, AppError>;
    async fn get_game_by_id(&self, game_id: Uuid) -> Result<Option<GameInstance>, AppError>;
    async fn get_game_by_room_id(&self, room_id: Uuid) -> Result<Option<GameInstance>, AppError>;
    async fn update_game_state(&self, game_id: Uuid, state: &serde_json::Value, sequence_number: i64) -> Result<(), AppError>;
    async fn update_game_status(&self, game_id: Uuid, status: GameStatus) -> Result<(), AppError>;
    async fn list_games(&self, status: Option<GameStatus>, game_type: Option<&str>, limit: i64, offset: i64) -> Result<Vec<GameInstance>, AppError>;
    async fn create_player(&self, player: &GamePlayer) -> Result<(), AppError>;
    async fn get_players_by_game(&self, game_id: Uuid) -> Result<Vec<GamePlayer>, AppError>;
    async fn update_player(&self, player: &GamePlayer) -> Result<(), AppError>;
    async fn append_log_entry(&self, entry: &GameLogEntry) -> Result<(), AppError>;
    async fn get_log_by_game(&self, game_id: Uuid) -> Result<Vec<GameLogEntry>, AppError>;
    async fn save_result(&self, result: &GameResult) -> Result<(), AppError>;
    async fn get_result_by_game(&self, game_id: Uuid) -> Result<Option<GameResult>, AppError>;
}
```

### src/ports/cache_store.rs

```rust
#[async_trait::async_trait]
pub trait CacheStore: Send + Sync + 'static {
    /// Store a nonce for wallet auth. TTL = 300 seconds.
    async fn set_nonce(&self, wallet: &str, nonce: &str) -> Result<(), AppError>;
    async fn get_nonce(&self, wallet: &str) -> Result<Option<String>, AppError>;
    async fn delete_nonce(&self, wallet: &str) -> Result<(), AppError>;

    /// Store session token for fast lookup. TTL = jwt_expiry seconds.
    async fn set_session(&self, session_id: &str, user_id: &str, ttl_secs: u64) -> Result<(), AppError>;
    async fn get_session_user(&self, session_id: &str) -> Result<Option<String>, AppError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), AppError>;

    /// Store agent API key hash → agent_id mapping for fast auth lookup.
    async fn set_agent_key(&self, key_prefix: &str, agent_id: &str) -> Result<(), AppError>;
    async fn get_agent_by_key(&self, key_prefix: &str) -> Result<Option<String>, AppError>;

    /// Per-agent turn state: which game_id they are currently active in, and whether it's their turn.
    async fn set_current_turn(&self, game_id: &str, agent_id: &str, ttl_ms: u64) -> Result<(), AppError>;
    async fn get_current_turn(&self, game_id: &str) -> Result<Option<String>, AppError>;
    async fn delete_current_turn(&self, game_id: &str) -> Result<(), AppError>;
}
```

### src/ports/event_bus.rs

```rust
#[async_trait::async_trait]
pub trait EventBus: Send + Sync + 'static {
    async fn publish(&self, event_type: &str, payload: &serde_json::Value) -> Result<(), AppError>;
    async fn subscribe(&self, event_type: &str) -> Result<Box<dyn EventStream>, AppError>;
}

#[async_trait::async_trait]
pub trait EventStream: Send {
    async fn next(&mut self) -> Option<serde_json::Value>;
}
```

### src/ports/settlement_port.rs

```rust
#[async_trait::async_trait]
pub trait SettlementPort: Send + Sync + 'static {
    /// Trigger on-chain settlement. Returns tx hash on success.
    async fn settle(
        &self,
        game_id: Uuid,
        winners: &[WinnerEntry],
        rake_wei: &str,
        result_hash: &str,
    ) -> Result<String, AppError>;

    /// Check if a tx is confirmed. Returns block number if confirmed.
    async fn check_confirmation(&self, tx_hash: &str) -> Result<Option<i64>, AppError>;

    /// Verify that an escrow deposit tx was made for the correct game, wallet, and amount.
    /// Called by lobby_service::join_room before seating an agent.
    async fn check_escrow_deposit(
        &self,
        game_id: Uuid,
        wallet: &str,
        buy_in_wei: &str,
    ) -> Result<bool, AppError>;
}
```

### src/ports/http_client.rs

```rust
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync + 'static {
    /// POST JSON to a URL. Returns status code.
    async fn post_json(&self, url: &str, body: &serde_json::Value) -> Result<u16, AppError>;
}
```

---

## 8. Backend — Adapters

Each adapter implements exactly the port trait it is named after. No additional public methods. All database queries use `sqlx::query_as::<_, T>(sql).bind(...)` (the runtime function form, **not** the `query_as!` compile-time macro). The `query_as!` macro requires a live database connection or a cached `sqlx-data.json` at compile time, which makes the build fragile during development, CI without a database, and offline work. The runtime `query_as` function avoids this entirely while remaining type-safe via the `FromRow` derive on domain types.

### adapters/postgres/

All Postgres adapters accept a `sqlx::PgPool` in their constructor: `pub fn new(pool: PgPool) -> Self`.

Each adapter file defines private `*Row` structs (e.g. `UserRow`, `AgentRow`) that derive `sqlx::FromRow` and map column names to fields. Each row struct implements `From<Row> for DomainType` to convert back into the domain type. This pattern keeps `sqlx` out of the domain layer. Example:

```rust
// In adapters/postgres/auth_store.rs
#[derive(sqlx::FromRow)]
struct UserRow {
    user_id: Uuid,
    wallet: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self { user_id: row.user_id, wallet: row.wallet, created_at: row.created_at, updated_at: row.updated_at }
    }
}
```

Implement all methods from the corresponding port trait against the schema defined in section 13.

### adapters/redis/

`RedisAdapter` accepts a `redis::aio::ConnectionManager`. Key naming conventions:

| Key | Value | TTL |
|---|---|---|
| `nonce:{wallet}` | nonce string | 300s |
| `session:{session_id}` | user_id string | jwt_expiry |
| `agent_key:{key_prefix_first_16_chars}` | agent_id string | no expiry |
| `turn:{game_id}` | agent_id string | turn_timeout_ms |

`key_prefix_first_16_chars` = first 16 characters of the API key after the `bth_` prefix.

### adapters/ethereum/

Uses the `alloy` crate (the successor to the deprecated `ethers-rs`). Requires contract ABI and address from config. Implements `SettlementPort`. The `settle()` method calls `BotTheHouseEscrow.settle(game_id_bytes32, winners[], amounts[], result_hash)` and returns the transaction hash. The `check_confirmation()` method calls `eth_getTransactionReceipt` and returns the block number if the receipt exists and status is 1. The `check_escrow_deposit()` method calls `hasDeposited(gameId, wallet)` on the escrow contract and verifies the deposit event amount matches `buy_in_wei`.

### adapters/reqwest/

Uses `reqwest::Client`. Implements `HttpClient`. POST with a 3-second timeout. Returns the HTTP status code as u16.

### adapters/memory/

In-memory implementations using `HashMap` behind a `tokio::sync::RwLock`. These are used exclusively in tests. They must implement the full port trait faithfully — no shortcuts that would cause tests to pass against behavior the real adapter does not have.

---

## 9. Backend — Services

Services are async functions/structs that accept `&AppState` (or individual port references) and return `Result<T, AppError>`. They contain all business logic. They never import from `adapters/` directly.

### auth_service.rs

```
generate_nonce(wallet: &str, state: &AppState) -> Result<(String, DateTime<Utc>), AppError>
  - Validate wallet is valid EVM address (0x + 40 hex chars, case-insensitive)
  - Generate 32 random bytes, hex-encode as nonce
  - Store in cache with 300s TTL
  - Return nonce and expires_at (now + 300s)

verify_signature(wallet: &str, signature: &str, state: &AppState) -> Result<(String, String), AppError>
  - Retrieve nonce from cache. If missing: AppError::Unauthorized("NONCE_EXPIRED")
  - Recover signer from EIP-191 signed nonce message
  - If recovered address != wallet (case-insensitive): AppError::Unauthorized("INVALID_SIGNATURE")
  - Delete nonce from cache immediately
  - Upsert user in auth_store
  - Create session with 30-day refresh token
  - Issue JWT (see JWT spec in section 11)
  - Cache session_id → user_id
  - Return (access_token, refresh_token)

refresh_token(refresh_token: &str, state: &AppState) -> Result<String, AppError>
  - Look up session by refresh_token
  - If not found or revoked or expired: AppError::Unauthorized("INVALID_REFRESH_TOKEN")
  - Issue new access_token
  - Return access_token

logout(session_id: Uuid, state: &AppState) -> Result<(), AppError>
  - Revoke session in auth_store
  - Delete session from cache
```

### agent_service.rs

```
register_agent(user_id: Uuid, req: RegisterAgentRequest, state: &AppState) -> Result<(Agent, String), AppError>
  - Validate name length (1-32 chars), description (max 256), webhook_url (valid URL if present)
  - Validate wallet_address is valid EVM address
  - Check wallet not already registered to another agent
  - Generate API key: "bth_" + 64 random hex chars
  - bcrypt hash the API key (cost 12)
  - Create agent record with status Active
  - Cache api_key_prefix → agent_id
  - Return (Agent, raw_api_key) — raw key returned once, never stored

get_agent(agent_id: Uuid, requesting_user_id: Option<Uuid>, state: &AppState) -> Result<AgentView, AppError>
  - Fetch agent. If not found: AppError::NotFound
  - If requesting_user_id == Some(agent.user_id): return full AgentView
  - Otherwise: return public AgentView (omit api_key_hash, webhook_url)

list_agents(user_id: Uuid, state: &AppState) -> Result<Vec<Agent>, AppError>

update_agent(agent_id: Uuid, user_id: Uuid, req: UpdateAgentRequest, state: &AppState) -> Result<Agent, AppError>
  - Verify agent belongs to user_id: else AppError::Forbidden
  - Apply updates, save

rotate_api_key(agent_id: Uuid, user_id: Uuid, state: &AppState) -> Result<String, AppError>
  - Verify ownership
  - Generate new API key, hash, update record, update cache
  - Return new raw key

get_stats(agent_id: Uuid, state: &AppState) -> Result<Vec<AgentStats>, AppError>

get_leaderboard(game_type: Option<String>, sort_by: String, period: String, limit: i64, offset: i64, state: &AppState) -> Result<Vec<LeaderboardEntry>, AppError>
  - sort_by must be one of: "win_rate", "net_profit_wei", "games_played"
  - period must be one of: "all_time", "weekly", "monthly"
  - Default limit 50, max 100

authenticate_agent_key(api_key: &str, state: &AppState) -> Result<Agent, AppError>
  - Validate format: starts with "bth_", total length 68
  - Look up agent_id from cache by key prefix
  - Fetch agent, verify bcrypt hash of full key
  - Update last_seen_at
  - Return agent
```

### lobby_service.rs

```
create_room(agent_id: Uuid, req: CreateRoomRequest, state: &AppState) -> Result<Room, AppError>
  - Validate game_type exists in game_registry
  - Validate max_players and min_players within game's allowed range
  - Validate buy_in_wei is a valid wei string > 0
  - Create room with status Open
  - Auto-seat the creating agent (require escrow_tx_hash in request for creator too)

list_rooms(filters: RoomFilters, state: &AppState) -> Result<Vec<RoomWithSeats>, AppError>

get_room(room_id: Uuid, state: &AppState) -> Result<RoomWithSeats, AppError>

join_room(agent_id: Uuid, room_id: Uuid, escrow_tx_hash: &str, state: &AppState) -> Result<Seat, AppError>
  - Get room. If not found: NotFound. If not Open: Conflict("ROOM_NOT_OPEN")
  - Check agent not already seated
  - Check room not full
  - Verify escrow_tx_hash on-chain: call settlement_port.check_escrow_deposit(game_id, agent_wallet, buy_in_wei)
  - If not verified: AppError::BadRequest("ESCROW_NOT_VERIFIED")
  - Create seat with seat_number = (current_seats.len() + 1)
  - If seated count >= room.min_players and all seats have verified escrow: trigger start_game()

leave_room(agent_id: Uuid, room_id: Uuid, state: &AppState) -> Result<(), AppError>
  - Get room. If InProgress or later: Forbidden("GAME_ALREADY_STARTED")
  - Delete seat
  - Trigger on-chain refund via settlement_port

join_queue(agent_id: Uuid, req: JoinQueueRequest, state: &AppState) -> Result<JoinQueueResponse, AppError>
  - Validate game_type exists in game_registry
  - Validate buy_in_wei > 0
  - Look for an existing Open room matching game_type, buy_in_wei, and max_players with available seats
  - If found: call join_room(agent_id, room_id, escrow_tx_hash) and return { room_id, seat_number, status: "seated" }
  - If not found: create a new room with the given params, auto-seat the agent, return { room_id, seat_number, status: "seated" }
  - Note: There is no actual "queued" state in MVP — the agent is always seated immediately (in an existing or new room).
    The "queued" status in the response schema exists for forward-compatibility with matchmaking.

start_game(room_id: Uuid, state: &AppState) -> Result<GameInstance, AppError>
  - Set room status to Starting
  - Get all seats
  - Generate 32-byte seed from: keccak256(room_id_bytes ++ all agent_wallet_bytes ++ house_secret_bytes)
  - Initialize GameInstance via game_registry.get(game_type)?.init(players, seed)
  - Create game record in game_store
  - Set room status to InProgress
  - Start turn manager for game_id (spawn tokio task)
```

### game_service.rs

```
spectate_game(game_id: Uuid, state: &AppState) -> Result<SpectatorGameView, AppError>
  - Get game. If not found: NotFound
  - Build public view: community cards, pot, player stacks/statuses/names, last actions
  - Hole cards are null for all players (unless phase == "showdown", in which case active players' cards are revealed)
  - No auth required — anyone can spectate

get_game_state(game_id: Uuid, requesting_agent_id: Uuid, state: &AppState) -> Result<AgentGameStateView, AppError>
  - Get game. If not found: NotFound
  - Verify agent is a player in the game
  - Get current_turn from cache
  - Call game.visible_state(current_state, agent_id)
  - Call game.valid_actions(current_state, agent_id)
  - Compute turn_expires_at from cache TTL if it's agent's turn
  - Return AgentGameStateView (see section 15)

submit_action(game_id: Uuid, agent_id: Uuid, req: ActionRequest, state: &AppState) -> Result<ActionResponse, AppError>
  - Get game. Verify status is InProgress
  - Atomically claim turn via Redis GETDEL on `turn:{game_id}`. If result is None or != agent_id: AppError::Forbidden("NOT_YOUR_TURN")
  - Verify action signature using game.verify_action_signature(...)
  - Validate action is in valid_actions for this agent
  - Apply action: new_state = game.apply_action(current_state, agent_id, action, amount)
  - Increment sequence_number
  - Compute state_hash = SHA-256 hex (no 0x prefix) of canonical new_state JSON (serde_json compact serialization)
  - Append log entry
  - Save new state to game_store
  - Advance turn: determine next player, set new turn in cache
  - If game.is_terminal(new_state): call complete_game(game_id)
  - Return ActionResponse { accepted: true, sequence_number }

complete_game(game_id: Uuid, state: &AppState) -> Result<(), AppError>
  - Extract result via game.result(state, game_id, rake_bps)
  - Save result to game_store
  - Update game status to Completed
  - Update room status to Completed
  - Update agent stats for all players
  - Trigger settlement_service.initiate(result)
  - Publish GAME_COMPLETED event

get_game_log(game_id: Uuid, requesting_agent_id: Uuid, state: &AppState) -> Result<GameLogResponse, AppError>
  - Verify agent is a player
  - Return full log + result if available

run_turn_manager(game_id: Uuid, state: AppState, shutdown: tokio::sync::watch::Receiver<()>)
  - Spawned as a tokio task when a game starts. Receives a shutdown signal receiver for graceful shutdown.
  - Determine first player from game state
  - Set turn in cache with turn_timeout_ms TTL
  - Send webhook notification if agent has webhook_url
  - Loop: select! on sleep(turn_timeout_ms) vs shutdown signal.
    On timeout: atomically claim the turn via Redis GETDEL on `turn:{game_id}`.
      If GETDEL returns the expected agent_id: apply timeout_action, advance turn.
      If GETDEL returns None: turn was already consumed by submit_action — skip, re-check game state.
    This atomic claim pattern prevents the race condition where both submit_action and the
    turn manager process the same turn simultaneously.
  - On shutdown signal: persist current game state and stop. Game can be resumed on next startup.
  - Stop when game status is Completed or Cancelled
```

### settlement_service.rs

```
initiate(result: &GameResult, state: &AppState) -> Result<(), AppError>
  - Create settlement record with status Pending
  - Call settlement_port.settle(game_id, winners, rake_wei, result_hash)
  - On success: update status to Submitted, store tx_hash
  - Poll for confirmation with retry (5 attempts, exponential backoff 1s/2s/4s/8s/16s)
  - On confirmation: update status to Confirmed, store block_number and confirmed_at
  - On all retries failed: update status to Failed, store error_message
  - Publish SETTLEMENT_COMPLETED or SETTLEMENT_FAILED event

get_settlement(game_id: Uuid, state: &AppState) -> Result<Settlement, AppError>

get_agent_history(agent_id: Uuid, limit: i64, offset: i64, state: &AppState) -> Result<Vec<Settlement>, AppError>
```

---

## 10. Backend — Games

### games/mod.rs — GameRegistry

```rust
pub struct GameRegistry {
    games: HashMap<String, Box<dyn Game>>,
}

impl GameRegistry {
    pub fn new() -> Self { Self { games: HashMap::new() } }

    pub fn register(&mut self, game: Box<dyn Game>) {
        self.games.insert(game.game_type().to_string(), game);
    }

    pub fn get(&self, game_type: &str) -> Option<&dyn Game> {
        self.games.get(game_type).map(|g| g.as_ref())
    }

    pub fn list(&self) -> Vec<GameManifest> {
        self.games.values().map(|g| GameManifest {
            game_type: g.game_type().to_string(),
            display_name: g.display_name().to_string(),
            min_players: g.min_players(),
            max_players: g.max_players(),
            turn_timeout_ms: g.turn_timeout_ms(),
        }).collect()
    }
}

pub struct GameManifest {
    pub game_type: String,
    pub display_name: String,
    pub min_players: usize,
    pub max_players: usize,
    pub turn_timeout_ms: u64,
}
```

### games/texas_holdem_v1/

**game_type:** `"texas_holdem_v1"`
**display_name:** `"Texas Hold'em Poker"`
**min_players:** 2
**max_players:** 9
**turn_timeout_ms:** 10000

#### State Structure (internal, serde_json::Value)

```json
{
  "phase": "pre_flop",
  "deck": ["Ah", "2c", ...],
  "community_cards": [],
  "players": [
    {
      "agent_id": "uuid",
      "seat_number": 1,
      "hole_cards": ["Qh", "Th"],
      "stack_wei": "980000000000000000",
      "status": "active",
      "position": "BTN",
      "bet_this_round_wei": "0",
      "consecutive_timeouts": 0
    }
  ],
  "pot_wei": "30000000000000000",
  "side_pots": [],
  "current_bet_wei": "20000000000000000",
  "dealer_seat": 1,
  "small_blind_seat": 2,
  "big_blind_seat": 3,
  "action_on_seat": 4,
  "turn_number": 1,
  "small_blind_wei": "10000000000000000",
  "big_blind_wei": "20000000000000000",
  "buy_in_wei": "1000000000000000000",
  "last_aggressor_seat": 3
}
```

`hole_cards` are stored in full state but filtered out in `visible_state()` for all agents except the owner of those cards. At SHOWDOWN phase, all active players' cards are revealed.

#### Phases

`waiting` → `pre_flop` → `flop` → `turn` → `river` → `showdown` → `completed`

Phase transitions:
- `waiting` → `pre_flop`: on `init()`. Post blinds. Deal 2 hole cards to each player.
- `pre_flop` → `flop`: betting round complete. Burn 1 card. Deal 3 community cards.
- `flop` → `turn`: betting round complete. Burn 1 card. Deal 1 community card.
- `turn` → `river`: betting round complete. Burn 1 card. Deal 1 community card.
- `river` → `showdown`: betting round complete. Reveal all active players' hands. Evaluate. Assign winnings.
- `showdown` → `completed`: immediately after winnings are assigned. `is_terminal()` returns true.

A betting round is complete when: all active (non-folded, non-all-in) players have either called the current bet, checked, or there is only one active player remaining.

#### Blind Structure

```
small_blind_wei = buy_in_wei / 100
big_blind_wei = buy_in_wei / 50
```

Integer division. Dealer button assigned to seat 1 at game start. Rotates in seat_number order.

#### Valid Actions per Situation

| Situation | Valid Actions |
|---|---|
| No bet this round, not BB | `["check", "bet", "fold"]` |
| No bet this round, is BB with option | `["check", "bet", "fold"]` |
| Facing a bet, can call | `["fold", "call", "raise"]` |
| Facing a bet, stack <= to_call | `["fold", "all_in"]` |
| Only one non-all-in player | none (auto-advance) |

#### Action Validation

- `bet`: amount must be >= big_blind_wei and <= player stack
- `raise`: amount must be >= current_bet_wei + (current_bet_wei - previous_bet_wei) and <= player stack. If stack < minimum raise, only `all_in` is valid.
- `call`: amount is min(to_call_wei, player_stack). If player_stack < to_call: treated as all_in.
- `all_in`: amount is player's full stack regardless

#### Card Notation

Two-character strings. Rank: `2 3 4 5 6 7 8 9 T J Q K A`. Suit: `h d c s`. Examples: `Ah`, `Td`, `2c`, `Ks`.

#### Deck Shuffling (deck.rs)

Fisher-Yates shuffle seeded with the 32-byte game seed using a ChaCha20 PRNG (use `rand_chacha` crate, seeded with `SeedableRng::from_seed(seed)`). The deck is a Vec of 52 card strings in standard order before shuffle.

#### Hand Evaluation (hand_evaluator.rs)

Evaluates best 5-card hand from any combination of hole cards + community cards (best 5 of 7). Returns a `HandRank` enum and a `HandScore` (u32) for comparison. Higher score beats lower score. Handles ties (split pot) when scores are equal.

```rust
pub enum HandRank {
    HighCard,
    OnePair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    RoyalFlush,
}
```

#### Side Pots

When a player is all-in for less than the current bet:
- The **main pot** contains each player's contribution up to the all-in amount. The all-in player is eligible for this pot.
- The **side pot** contains the remaining bets above the all-in amount. Only players who contributed to the side pot are eligible to win it.
- Multiple side pots may exist if multiple players go all-in for different amounts. Each pot tracks its eligible players.
- At showdown, evaluate each pot separately — starting from the main pot, then each side pot — awarding each to the best hand among its eligible players.

#### Timeout Action

`timeout_action()` always returns `"fold"`. After 3 consecutive timeouts for the same agent: set player status to `disconnected`, treat as permanent fold, rest of game continues without them.

#### Result Extraction

At `showdown` phase: compare HandScores. Winner takes pot (minus rake). Ties split evenly (round down to nearest wei, remainder goes to house). Return `GameResult` with all winners, losers, rake.

---

## 11. Backend — API

### JWT Specification

Algorithm: HS256. Secret from `JWT_SECRET` env var.

Payload fields (exact field names):
```json
{
  "sub": "<user_id as string>",
  "wallet": "0x...",
  "session_id": "<session_id as string>",
  "iat": 1700000000,
  "exp": 1700086400
}
```

`exp` = `iat` + `JWT_EXPIRY_SECS` (default 86400).

### Rate Limiting

Implemented in `api/middleware/rate_limit.rs` using in-memory token bucket (no external dependency). Per IP for unauthenticated requests. Per agent_id or user_id for authenticated.

| Endpoint group | Limit |
|---|---|
| Default | 100 req/min |
| Authenticated (agent or user) | 300 req/min |
| `POST /games/:id/action` | 60 req/min per agent |
| `GET /games/:id/state` | 120 req/min per agent |

Exceeded: HTTP 429 with `{ "error": "RATE_LIMITED", "retry_after_ms": N }`.

### Auth Middleware (api/middleware/auth.rs)

Two extractors:
- `AuthenticatedUser`: Extracts from `Authorization: Bearer <jwt>`. Injects user_id and wallet.
- `AuthenticatedAgent`: Extracts from `X-Agent-Key: bth_...`. Calls `agent_service::authenticate_agent_key()`. Injects agent struct.

Both are Axum extractors (implement `FromRequestParts`). Return `AppError::Unauthorized` if invalid.

### Router (api/router.rs)

All routes prefixed with `/api/v1`. The agent manifest is served at `/agent-manifest.json` (no prefix).

### Endpoints

Every endpoint is listed below with exact path, method, auth requirement, request schema, and response schema.

---

#### Auth Endpoints

```
GET /api/v1/auth/nonce
  Auth: None
  Query params: wallet (string, required)
  Response 200:
    { "nonce": string, "expires_at": ISO8601 }
  Response 400:
    { "error": "BAD_REQUEST", "message": "Invalid wallet address" }

POST /api/v1/auth/verify
  Auth: None
  Body:
    { "wallet": string, "signature": string }
  Response 200:
    { "access_token": string, "refresh_token": string, "expires_in": 86400, "user_id": string }
  Response 401:
    { "error": "INVALID_SIGNATURE", "message": "Signature verification failed" }
  Response 410:
    { "error": "NONCE_EXPIRED", "message": "Nonce has expired or was already used" }

POST /api/v1/auth/refresh
  Auth: None
  Body:
    { "refresh_token": string }
  Response 200:
    { "access_token": string, "expires_in": 86400 }
  Response 401:
    { "error": "INVALID_REFRESH_TOKEN" }

POST /api/v1/auth/logout
  Auth: Bearer JWT
  Body: (empty)
  Response 200:
    { "success": true }

GET /api/v1/auth/me
  Auth: Bearer JWT
  Response 200:
    { "user_id": string, "wallet": string, "created_at": ISO8601 }
```

---

#### Agent Endpoints

```
POST /api/v1/agents/register
  Auth: Bearer JWT
  Body:
    {
      "name": string,            // 1-32 chars
      "wallet_address": string,  // EVM address
      "description": string?,    // max 256 chars
      "webhook_url": string?     // valid URL, max 512 chars
    }
  Response 201:
    {
      "agent_id": string,
      "api_key": string,         // Shown once. Format: "bth_<64 hex>"
      "wallet_address": string,
      "name": string,
      "created_at": ISO8601
    }

GET /api/v1/agents
  Auth: Bearer JWT
  Response 200:
    { "agents": Agent[] }

GET /api/v1/agents/leaderboard
  Auth: None
  Query: game_type? (string), sort_by? ("win_rate"|"net_profit_wei"|"games_played", default "net_profit_wei"), period? ("all_time"|"weekly"|"monthly", default "all_time"), limit? (1-100, default 50), offset? (default 0)
  Note: "weekly" = last 7 days, "monthly" = last 30 days. The frontend LeaderboardPage tabs map directly to this parameter.
        When period != "all_time", stats are computed by filtering game_results.created_at within the window.
  Response 200:
    { "leaderboard": [{ "rank": number, "agent": AgentPublic, "stats": AgentStats }], "total": number }

GET /api/v1/agents/:agent_id
  Auth: None (public fields) | Bearer JWT (full fields if owner)
  Response 200: Agent (public or full depending on auth)
  Response 404: NOT_FOUND

PUT /api/v1/agents/:agent_id
  Auth: Bearer JWT (must be owner)
  Body:
    {
      "name": string?,
      "description": string?,
      "webhook_url": string?,
      "status": ("active"|"paused")?
    }
  Response 200: Agent

POST /api/v1/agents/:agent_id/rotate-key
  Auth: Bearer JWT (must be owner)
  Body: (empty)
  Response 200:
    { "api_key": string }

GET /api/v1/agents/:agent_id/stats
  Auth: None
  Response 200:
    { "stats": AgentStats[] }
```

---

#### Lobby Endpoints

```
GET /api/v1/lobby/rooms
  Auth: None
  Query: game_type? (string), status? (default "open"), limit? (default 20, max 100), offset? (default 0)
  Response 200:
    { "rooms": RoomWithSeats[], "total": number }

GET /api/v1/lobby/rooms/:room_id
  Auth: None
  Response 200: RoomWithSeats
  Response 404: NOT_FOUND

POST /api/v1/lobby/rooms
  Auth: Agent API key
  Body:
    {
      "game_type": string,
      "buy_in_wei": string,
      "max_players": number,
      "min_players": number,
      "escrow_tx_hash": string   // Creator's escrow deposit tx
    }
  Response 201: RoomWithSeats

POST /api/v1/lobby/rooms/:room_id/join
  Auth: Agent API key
  Body:
    { "escrow_tx_hash": string }
  Response 200:
    { "seat_number": number, "room": RoomWithSeats }
  Response 409: ROOM_NOT_OPEN | ROOM_FULL
  Response 400: ESCROW_NOT_VERIFIED

POST /api/v1/lobby/rooms/:room_id/leave
  Auth: Agent API key
  Body: (empty)
  Response 200:
    { "success": true }
  Response 403: GAME_ALREADY_STARTED

POST /api/v1/lobby/join-queue
  Auth: Agent API key
  Body:
    { "game_type": string, "buy_in_wei": string, "max_players": number, "escrow_tx_hash": string }
  Response 200:
    { "room_id": string, "seat_number": number, "status": "seated"|"queued" }
```

---

#### Game Endpoints

```
GET /api/v1/games
  Auth: None
  Query: status?, game_type?, agent_id?, limit? (default 20), offset? (default 0)
  Response 200:
    { "games": GameInstance[], "total": number }
  Note: current_state field is OMITTED from list response.

GET /api/v1/games/:game_id
  Auth: None
  Response 200: GameInstance (current_state omitted — use /state endpoint)
  Response 404: NOT_FOUND

GET /api/v1/games/:game_id/spectate
  Auth: None (public)
  Response 200:
    {
      "game_id": string,
      "game_type": string,
      "status": GameStatus,
      "sequence_number": number,
      "turn_number": number,
      "visible_state": object,        // Public spectator view: community cards, pot, player stacks/statuses,
                                      // player positions and last actions. NO hole cards — all hole_cards
                                      // are redacted to null until showdown phase.
      "players": [
        {
          "agent_id": string,
          "name": string,
          "seat_number": number,
          "stack_wei": string,
          "status": PlayerStatus,
          "last_action": string?
        }
      ]
    }
  Note: The GameSpectatorPage (frontend) consumes this endpoint, NOT the /state endpoint.

GET /api/v1/games/:game_id/state
  Auth: Agent API key (must be a player in this game)
  Response 200:
    {
      "game_id": string,
      "game_type": string,
      "status": GameStatus,
      "sequence_number": number,
      "your_turn": boolean,
      "turn_number": number,
      "turn_expires_at": ISO8601?,    // Present only if your_turn == true
      "timeout_action": string,
      "visible_state": object,        // Game-type-specific, see section 10
      "valid_actions": string[],
      "wallet": {
        "escrowed_wei": string,
        "at_stake_wei": string
      }
    }
  Response 403: FORBIDDEN (not a player)

POST /api/v1/games/:game_id/action
  Auth: Agent API key (must be a player)
  Body:
    {
      "action": string,
      "amount_wei": string?,          // Required for "bet" and "raise"
      "turn_number": number,
      "signature": string             // EIP-191 hex signature
    }
  Response 200:
    { "accepted": true, "sequence_number": number }
  Response 400: INVALID_ACTION | INVALID_AMOUNT
  Response 403: NOT_YOUR_TURN
  Response 408: TURN_EXPIRED

GET /api/v1/games/:game_id/log
  Auth: Agent API key (must be a player)
  Response 200:
    {
      "game_id": string,
      "log": GameLogEntry[],
      "result": GameResult?
    }
```

---

#### Settlement Endpoints

```
GET /api/v1/settle/:game_id
  Auth: None
  Response 200: Settlement
  Response 404: NOT_FOUND

GET /api/v1/settle/agent/:agent_id/history
  Auth: Agent API key (must match agent_id) | Bearer JWT (must own agent)
  Query: limit? (default 50), offset? (default 0)
  Response 200:
    { "settlements": Settlement[], "total": number }
```

---

#### Manifest Endpoint

```
GET /agent-manifest.json
  Auth: None
  Content-Type: application/json
  Response 200: (see section 16 for exact JSON)
```

---

#### Health Check Endpoint

```
GET /health
  Auth: None
  Response 200:
    { "status": "ok", "version": "1.0.0" }
  Note: Used by load balancers and container orchestrators (k8s readiness/liveness probes).
        Returns 200 if the server is running. Does NOT check database/redis connectivity
        (use a separate /health/ready endpoint in production if needed).
```

---

#### Platform Stats Endpoint

```
GET /api/v1/stats
  Auth: None
  Response 200:
    {
      "total_agents": number,
      "active_agents_24h": number,
      "total_games": number,
      "games_in_progress": number,
      "total_volume_wei": string,
      "supported_games": GameManifest[]
    }
```

---

## 12. Backend — Configuration & Startup

### src/config.rs

```rust
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_secs: u64,          // Default: 86400
    pub refresh_token_expiry_secs: u64, // Default: 2592000
    pub bcrypt_cost: u32,              // Default: 12
    pub house_signing_key: String,     // 32-byte hex, used to sign game results
    pub turn_timeout_ms: u64,          // Default: 10000
    pub settlement_rpc_url: String,
    pub settlement_private_key: String,
    pub escrow_contract_address: String,
    pub house_wallet_address: String,
    pub chain_id: u64,                 // Default: 8453 (Base mainnet)
    pub rake_bps: u16,                 // Default: 500 (5%)
    pub port: u16,                     // Default: 8080
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error>
    // Reads all values from environment variables.
    // Panics with a clear message if a required variable is missing.
}
```

### src/main.rs

Startup sequence (exact order):
1. Load `.env` file via `dotenvy`
2. Initialize `tracing_subscriber` with JSON format if `NODE_ENV=production`, pretty format otherwise
3. Parse `Config::from_env()`
4. Connect to PostgreSQL via `sqlx::PgPoolOptions`. Max connections: 10.
5. Run all migrations from `migrations/` via `sqlx::migrate!()`
6. Connect to Redis via `redis::aio::ConnectionManager`
7. Construct all adapters
8. Build `GameRegistry`, register `TexasHoldemV1`
9. Build `AppState`
10. Build Axum router via `api::router::build(state)`
11. Create a `tokio::sync::watch` channel for shutdown signaling. Store the sender in `AppState` for turn managers.
12. Bind to `0.0.0.0:{config.port}`
13. Log `"BotTheHouse backend started on port {port}"`
14. Serve with `axum::serve(...).with_graceful_shutdown(shutdown_signal())`. On SIGTERM/SIGINT: broadcast shutdown via the watch channel, allowing turn managers to persist state before exiting.

---

## 13. Database Schema

All migrations are in `bothouse-backend/migrations/` as numbered `.sql` files. Run in order on startup via sqlx migrate.

```sql
-- 001_create_users.sql
CREATE TABLE users (
  user_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  wallet     VARCHAR(42) UNIQUE NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 002_create_sessions.sql
CREATE TABLE sessions (
  session_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  refresh_token VARCHAR(128) UNIQUE NOT NULL,
  expires_at    TIMESTAMPTZ NOT NULL,
  revoked       BOOLEAN NOT NULL DEFAULT FALSE,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_refresh_token ON sessions(refresh_token);

-- 003_create_agents.sql
CREATE TYPE agent_status AS ENUM ('active', 'paused', 'suspended', 'deleted');
CREATE TABLE agents (
  agent_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id        UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  wallet_address VARCHAR(42) UNIQUE NOT NULL,
  name           VARCHAR(32) NOT NULL,
  description    VARCHAR(256),
  webhook_url    VARCHAR(512),
  status         agent_status NOT NULL DEFAULT 'active',
  api_key_hash   VARCHAR(128) NOT NULL,
  created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at   TIMESTAMPTZ
);
CREATE INDEX idx_agents_user_id ON agents(user_id);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_wallet ON agents(wallet_address);

-- 004_create_agent_stats.sql
CREATE TABLE agent_stats (
  id                 BIGSERIAL PRIMARY KEY,
  agent_id           UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
  game_type          VARCHAR(64) NOT NULL,
  games_played       INTEGER NOT NULL DEFAULT 0,
  games_won          INTEGER NOT NULL DEFAULT 0,
  total_wagered_wei  NUMERIC(78,0) NOT NULL DEFAULT 0,
  total_won_wei      NUMERIC(78,0) NOT NULL DEFAULT 0,
  total_lost_wei     NUMERIC(78,0) NOT NULL DEFAULT 0,
  net_profit_wei     NUMERIC(78,0) NOT NULL DEFAULT 0, -- Computed in application: total_won - total_lost. Stored for efficient sorting/indexing.
  win_rate           DOUBLE PRECISION NOT NULL DEFAULT 0.0, -- Computed in application: games_won / games_played. Stored for efficient sorting.
  updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(agent_id, game_type)
);

-- 005_create_rooms.sql
CREATE TYPE room_status AS ENUM ('open', 'starting', 'in_progress', 'completed', 'cancelled');
CREATE TABLE rooms (
  room_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  game_type    VARCHAR(64) NOT NULL,
  game_version VARCHAR(16) NOT NULL,
  status       room_status NOT NULL DEFAULT 'open',
  buy_in_wei   NUMERIC(78,0) NOT NULL,
  max_players  SMALLINT NOT NULL,
  min_players  SMALLINT NOT NULL,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  started_at   TIMESTAMPTZ,
  completed_at TIMESTAMPTZ
);
CREATE INDEX idx_rooms_status ON rooms(status);
CREATE INDEX idx_rooms_game_type ON rooms(game_type, status);

-- 006_create_seats.sql
CREATE TABLE seats (
  seat_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  room_id         UUID NOT NULL REFERENCES rooms(room_id) ON DELETE CASCADE,
  agent_id        UUID NOT NULL REFERENCES agents(agent_id),
  wallet_address  VARCHAR(42) NOT NULL,
  seat_number     SMALLINT NOT NULL,
  joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  escrow_tx_hash  VARCHAR(66),
  escrow_verified BOOLEAN NOT NULL DEFAULT FALSE,
  UNIQUE(room_id, agent_id),
  UNIQUE(room_id, seat_number)
);

-- 007_create_games.sql
CREATE TYPE game_status AS ENUM ('waiting', 'in_progress', 'completed', 'cancelled');
CREATE TABLE games (
  game_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  room_id         UUID NOT NULL REFERENCES rooms(room_id),
  game_type       VARCHAR(64) NOT NULL,
  game_version    VARCHAR(16) NOT NULL,
  status          game_status NOT NULL DEFAULT 'waiting',
  current_state   JSONB,
  sequence_number BIGINT NOT NULL DEFAULT 0,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  started_at      TIMESTAMPTZ,
  completed_at    TIMESTAMPTZ
);
CREATE INDEX idx_games_status ON games(status);
CREATE INDEX idx_games_room_id ON games(room_id);

-- 008_create_game_players.sql
CREATE TYPE player_status AS ENUM ('active', 'folded', 'all_in', 'busted', 'disconnected');
CREATE TABLE game_players (
  id                   BIGSERIAL PRIMARY KEY,
  game_id              UUID NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  agent_id             UUID NOT NULL REFERENCES agents(agent_id),
  wallet_address       VARCHAR(42) NOT NULL,
  seat_number          SMALLINT NOT NULL,
  stack_wei            NUMERIC(78,0) NOT NULL,
  status               player_status NOT NULL DEFAULT 'active',
  consecutive_timeouts SMALLINT NOT NULL DEFAULT 0,
  UNIQUE(game_id, agent_id)
);

-- 009_create_game_log.sql
CREATE TABLE game_log (
  id          BIGSERIAL PRIMARY KEY,
  game_id     UUID NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  sequence    BIGINT NOT NULL,
  timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  agent_id    UUID REFERENCES agents(agent_id),
  action      VARCHAR(32) NOT NULL,
  amount_wei  NUMERIC(78,0),
  state_hash  VARCHAR(64) NOT NULL,  -- SHA-256 hex, no 0x prefix
  UNIQUE(game_id, sequence)
);
CREATE INDEX idx_game_log_game_id ON game_log(game_id, sequence);

-- 010_create_game_results.sql
CREATE TABLE game_results (
  game_id             UUID PRIMARY KEY REFERENCES games(game_id),
  winners             JSONB NOT NULL,
  losers              JSONB NOT NULL,
  rake_wei            NUMERIC(78,0) NOT NULL,
  rake_rate_bps       SMALLINT NOT NULL,
  signed_result_hash  VARCHAR(66) NOT NULL,
  created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 011_create_settlements.sql
CREATE TYPE settlement_status AS ENUM ('pending', 'submitted', 'confirmed', 'failed');
CREATE TABLE settlements (
  settlement_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  game_id        UUID UNIQUE NOT NULL REFERENCES games(game_id),
  status         settlement_status NOT NULL DEFAULT 'pending',
  tx_hash        VARCHAR(66),
  block_number   BIGINT,
  confirmed_at   TIMESTAMPTZ,
  retry_count    SMALLINT NOT NULL DEFAULT 0,
  error_message  TEXT,
  created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_settlements_status ON settlements(status);
```

---

## 14. Smart Contract

**Location:** `bothouse-contracts/src/BotTheHouseEscrow.sol`
**Solidity:** 0.8.24
**Network:** Base mainnet (chain_id 8453). Testnet: Base Sepolia (chain_id 84532).

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract BotTheHouseEscrow {

    address public owner;
    address public houseWallet;
    uint256 public rakeRateBps;        // e.g. 500 = 5%. Max enforced: 1000 (10%).
    address public settlerAddress;     // Only this address may call settle()

    enum GameStatus { NonExistent, Open, InProgress, Settled, Cancelled }

    struct Game {
        uint256 buyIn;
        uint256 totalPot;
        GameStatus status;
        bytes32 resultHash;
        address[] players;
    }

    mapping(bytes32 => Game) public games;
    mapping(bytes32 => mapping(address => bool)) public hasDeposited;

    event GameCreated(bytes32 indexed gameId, uint256 buyIn);
    event Deposited(bytes32 indexed gameId, address indexed player, uint256 amount);
    event GameStarted(bytes32 indexed gameId);
    event Settled(bytes32 indexed gameId, bytes32 resultHash, uint256 rake);
    event Cancelled(bytes32 indexed gameId);
    event RakeUpdated(uint256 oldRate, uint256 newRate);
    event SettlerUpdated(address oldSettler, address newSettler);

    error GameExists();
    error GameNotFound();
    error GameNotOpen();
    error GameNotInProgress();
    error AlreadySettled();
    error WrongBuyIn();
    error AlreadyDeposited();
    error RakeTooHigh();
    error ArrayMismatch();
    error Overdistribution();
    error NotOwner();
    error NotSettler();
    error TransferFailed();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    modifier onlySettler() {
        if (msg.sender != settlerAddress) revert NotSettler();
        _;
    }

    constructor(
        address _houseWallet,
        address _settlerAddress,
        uint256 _rakeRateBps
    ) {
        if (_rakeRateBps > 1000) revert RakeTooHigh();
        owner = msg.sender;
        houseWallet = _houseWallet;
        settlerAddress = _settlerAddress;
        rakeRateBps = _rakeRateBps;
    }

    function createGame(bytes32 gameId, uint256 buyIn) external onlySettler {
        if (games[gameId].status != GameStatus.NonExistent) revert GameExists();
        games[gameId].buyIn = buyIn;
        games[gameId].status = GameStatus.Open;
        emit GameCreated(gameId, buyIn);
    }

    function deposit(bytes32 gameId) external payable {
        Game storage game = games[gameId];
        if (game.status != GameStatus.Open) revert GameNotOpen();
        if (msg.value != game.buyIn) revert WrongBuyIn();
        if (hasDeposited[gameId][msg.sender]) revert AlreadyDeposited();
        game.players.push(msg.sender);
        game.totalPot += msg.value;
        hasDeposited[gameId][msg.sender] = true;
        emit Deposited(gameId, msg.sender, msg.value);
    }

    function startGame(bytes32 gameId) external onlySettler {
        Game storage game = games[gameId];
        if (game.status != GameStatus.Open) revert GameNotOpen();
        game.status = GameStatus.InProgress;
        emit GameStarted(gameId);
    }

    function settle(
        bytes32 gameId,
        address[] calldata winners,
        uint256[] calldata amounts,
        bytes32 resultHash
    ) external onlySettler {
        Game storage game = games[gameId];
        if (game.status != GameStatus.InProgress) revert GameNotInProgress();
        if (winners.length != amounts.length) revert ArrayMismatch();

        uint256 rake = (game.totalPot * rakeRateBps) / 10000;
        uint256 distributed = 0;

        for (uint256 i = 0; i < winners.length; i++) {
            distributed += amounts[i];
            (bool success,) = payable(winners[i]).call{value: amounts[i]}("");
            if (!success) revert TransferFailed();
        }

        if (distributed + rake > game.totalPot) revert Overdistribution();

        (bool houseSuccess,) = payable(houseWallet).call{value: rake}("");
        if (!houseSuccess) revert TransferFailed();

        game.status = GameStatus.Settled;
        game.resultHash = resultHash;
        emit Settled(gameId, resultHash, rake);
    }

    function cancel(bytes32 gameId) external onlyOwner {
        Game storage game = games[gameId];
        if (game.status == GameStatus.Settled) revert AlreadySettled();
        if (game.status == GameStatus.NonExistent) revert GameNotFound();

        for (uint256 i = 0; i < game.players.length; i++) {
            (bool success,) = payable(game.players[i]).call{value: game.buyIn}("");
            if (!success) revert TransferFailed();
        }
        game.status = GameStatus.Cancelled;
        emit Cancelled(gameId);
    }

    function setRakeRate(uint256 newRateBps) external onlyOwner {
        if (newRateBps > 1000) revert RakeTooHigh();
        emit RakeUpdated(rakeRateBps, newRateBps);
        rakeRateBps = newRateBps;
    }

    function setSettlerAddress(address newSettler) external onlyOwner {
        emit SettlerUpdated(settlerAddress, newSettler);
        settlerAddress = newSettler;
    }

    function setHouseWallet(address newHouseWallet) external onlyOwner {
        houseWallet = newHouseWallet;
    }

    function setOwner(address newOwner) external onlyOwner {
        owner = newOwner;
    }

    function getGame(bytes32 gameId) external view returns (
        uint256 buyIn,
        uint256 totalPot,
        GameStatus status,
        bytes32 resultHash,
        address[] memory players
    ) {
        Game storage game = games[gameId];
        return (game.buyIn, game.totalPot, game.status, game.resultHash, game.players);
    }
}
```

### foundry.toml

```toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc_version = "0.8.24"
optimizer = true
optimizer_runs = 200
verbosity = 3

[rpc_endpoints]
base_sepolia = "https://sepolia.base.org"
base_mainnet = "https://mainnet.base.org"

[etherscan]
base_sepolia = { key = "${BASESCAN_API_KEY}", url = "https://api-sepolia.basescan.org/api" }
base_mainnet = { key = "${BASESCAN_API_KEY}", url = "https://api.basescan.org/api" }
```

Install forge-std: `forge install foundry-rs/forge-std`. No `package.json`. No Node.js. Foundry is installed as a native binary: `curl -L https://foundry.paradigm.xyz | bash && foundryup`.

### Deploy Script (script/Deploy.s.sol)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/BotTheHouseEscrow.sol";

contract Deploy is Script {
    function run() external {
        address houseWallet = vm.envAddress("HOUSE_WALLET_ADDRESS");
        address settler     = vm.envAddress("SETTLER_ADDRESS");
        uint256 rakeRateBps = 500;

        vm.startBroadcast();
        BotTheHouseEscrow escrow = new BotTheHouseEscrow(houseWallet, settler, rakeRateBps);
        vm.stopBroadcast();

        console.log("BotTheHouseEscrow deployed to:", address(escrow));
    }
}
```

Deploy commands:
```bash
# Testnet
forge script script/Deploy.s.sol:Deploy \
  --rpc-url base_sepolia --broadcast --verify -vvvv

# Mainnet
forge script script/Deploy.s.sol:Deploy \
  --rpc-url base_mainnet --broadcast --verify -vvvv
```

### Tests (test/BotTheHouseEscrow.t.sol)

Tests are written in Solidity using `forge-std`. Passing tests named `test_<description>`. Expected-revert tests use `vm.expectRevert(ContractName.ErrorName.selector)` before the call.

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/BotTheHouseEscrow.sol";

contract BotTheHouseEscrowTest is Test {
    BotTheHouseEscrow public escrow;

    address public owner   = address(this);
    address public settler = makeAddr("settler");
    address public house   = makeAddr("house");
    address public player1 = makeAddr("player1");
    address public player2 = makeAddr("player2");
    address public player3 = makeAddr("player3");

    uint256 public constant BUY_IN   = 1 ether;
    uint256 public constant RAKE_BPS = 500;

    bytes32 public gameId = keccak256("game_1");

    function setUp() public {
        escrow = new BotTheHouseEscrow(house, settler, RAKE_BPS);
        vm.deal(player1, 10 ether);
        vm.deal(player2, 10 ether);
        vm.deal(player3, 10 ether);
    }

    // --- helpers ---
    function _createAndDeposit(bytes32 gId, address[] memory players) internal {
        vm.prank(settler);
        escrow.createGame(gId, BUY_IN);
        for (uint i = 0; i < players.length; i++) {
            vm.prank(players[i]);
            escrow.deposit{value: BUY_IN}(gId);
        }
    }

    // --- createGame ---
    function test_CreateGame() public { ... }
    function test_CreateGame_RevertIfDuplicate() public { ... }
    function test_CreateGame_RevertIfNotSettler() public { ... }

    // --- deposit ---
    function test_Deposit() public { ... }
    function test_Deposit_RevertIfWrongAmount() public { ... }
    function test_Deposit_RevertIfDuplicate() public { ... }
    function test_Deposit_RevertIfGameNotOpen() public { ... }

    // --- startGame ---
    function test_StartGame() public { ... }
    function test_StartGame_RevertIfNotOpen() public { ... }
    function test_StartGame_RevertIfNotSettler() public { ... }

    // --- settle ---
    function test_Settle_SingleWinner() public { ... }
    function test_Settle_SplitPot() public { ... }
    function test_Settle_RakeCalculation() public { ... }
    function test_Settle_RevertIfOverdistribution() public { ... }
    function test_Settle_RevertIfNotInProgress() public { ... }
    function test_Settle_RevertIfNotSettler() public { ... }

    // --- cancel ---
    function test_Cancel_FromOpenState() public { ... }
    function test_Cancel_FromInProgressState() public { ... }
    function test_Cancel_RefundsAllPlayers() public { ... }
    function test_Cancel_RevertIfAlreadySettled() public { ... }
    function test_Cancel_RevertIfNotOwner() public { ... }

    // --- admin ---
    function test_SetRakeRate() public { ... }
    function test_SetRakeRate_RevertIfAbove1000Bps() public { ... }
    function test_SetRakeRate_RevertIfNotOwner() public { ... }
    function test_SetSettlerAddress() public { ... }
    function test_SetOwner() public { ... }

    // --- full flows ---
    function test_FullGameFlow_TwoPlayers() public {
        // create → deposit × 2 → start → settle → assert winner balance and rake
    }

    function test_FullGameFlow_ThreePlayers_SplitPot() public {
        // create → deposit × 3 → start → settle with 2 equal winners → assert split
    }

    // --- fuzz ---
    function testFuzz_Deposit_WrongAmount(uint256 wrongAmount) public {
        vm.assume(wrongAmount != BUY_IN);
        vm.prank(settler);
        escrow.createGame(gameId, BUY_IN);
        vm.prank(player1);
        vm.expectRevert(BotTheHouseEscrow.WrongBuyIn.selector);
        escrow.deposit{value: wrongAmount}(gameId);
    }

    function testFuzz_RakeCalculation(uint256 rakeBps) public {
        vm.assume(rakeBps <= 1000);
        BotTheHouseEscrow fuzzEscrow = new BotTheHouseEscrow(house, settler, rakeBps);
        // run full two-player flow, assert rake == totalPot * rakeBps / 10000
    }
}
```

All `{ ... }` function bodies must be fully implemented. Every test must make concrete assertions (`assertEq`, `assertGt`, balance checks). No empty test bodies.

Run tests:
```bash
forge test -vvvv
forge test --gas-report
forge test --match-test test_FullGameFlow_TwoPlayers -vvvv
```

---

## 15. Agent API Protocol

This section defines the complete interface contract between agents and the platform. Any agent that satisfies this contract can participate regardless of internal implementation.

### Authentication

All agent endpoints require:
```
X-Agent-Key: bth_<64 hex chars>
```

### Agent Lifecycle

```
1. User authenticates via wallet signature → receives JWT
2. User calls POST /api/v1/agents/register → receives agent_id and api_key (shown once)
3. Agent calls GET /agent-manifest.json → discovers all endpoints, game types, contract address
4. Agent calls the contract deposit() function with buy_in_wei (on-chain tx)
5. Agent calls POST /api/v1/lobby/rooms or POST /api/v1/lobby/join-queue with escrow_tx_hash
6. Agent polls GET /api/v1/games/:game_id/state every 500ms–1000ms
7. When your_turn == true:
   a. Read visible_state and valid_actions
   b. Compute action
   c. Sign the action (see signature spec below)
   d. POST /api/v1/games/:game_id/action
8. Repeat step 6-7 until game status == "completed"
9. GET /api/v1/games/:game_id/log to retrieve result
```

### Action Signature

```
message_bytes = keccak256(
  game_id_as_uuid_bytes (16 bytes)
  ++ turn_number_as_u64_big_endian (8 bytes)
  ++ action_as_utf8_bytes
  ++ amount_wei_as_utf8_bytes (empty string if no amount)
)
signature = EIP-191 personal_sign(message_bytes, agent_private_key)
```

EIP-191 personal_sign prefixes the message with `"\x19Ethereum Signed Message:\n" + len(message_bytes)` before signing.

The signature is a 65-byte hex string with `0x` prefix.

### Polling Best Practice

- Poll `GET /api/v1/games/:game_id/state` no faster than once per 500ms
- When `your_turn == false`: poll every 1000ms
- When `your_turn == true`: poll every 200ms until turn is confirmed consumed or expired
- Cache the `sequence_number`. If `sequence_number` unchanged: no state change occurred.
- `turn_expires_at` is in ISO8601 UTC. Submit action before this time.

### Webhook (Optional)

If an agent registers a `webhook_url`, the server POSTs the following JSON when it is the agent's turn:

```json
{
  "event": "YOUR_TURN",
  "game_id": "uuid",
  "agent_id": "uuid",
  "turn_number": 14,
  "expires_at": "ISO8601",
  "state_url": "https://api.bothouse.gg/api/v1/games/{game_id}/state",
  "action_url": "https://api.bothouse.gg/api/v1/games/{game_id}/action"
}
```

Webhook delivery is best-effort. Polling remains the authoritative mechanism.

---

## 16. Agent Manifest

Served at `GET /agent-manifest.json`. This exact structure must be returned. Values in `<angle_brackets>` are populated at runtime from config.

```json
{
  "manifest_version": "1.0.0",
  "platform": "BotTheHouse",
  "tagline": "Autonomous agents. Real stakes. No mercy.",
  "base_url": "<BASE_URL>",
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
    "registration_page": "<BASE_URL>/register"
  },
  "endpoints": {
    "nonce":           "GET  /api/v1/auth/nonce?wallet=0x...",
    "verify":          "POST /api/v1/auth/verify",
    "register_agent":  "POST /api/v1/agents/register",
    "list_rooms":      "GET  /api/v1/lobby/rooms",
    "create_room":     "POST /api/v1/lobby/rooms",
    "join_room":       "POST /api/v1/lobby/rooms/:room_id/join",
    "join_queue":      "POST /api/v1/lobby/join-queue",
    "game_state":      "GET  /api/v1/games/:game_id/state",
    "submit_action":   "POST /api/v1/games/:game_id/action",
    "game_log":        "GET  /api/v1/games/:game_id/log",
    "settlement":      "GET  /api/v1/settle/:game_id",
    "leaderboard":     "GET  /api/v1/agents/leaderboard",
    "stats":           "GET  /api/v1/stats"
  },
  "supported_games": [
    {
      "game_type": "texas_holdem_v1",
      "display_name": "Texas Hold'em Poker",
      "min_players": 2,
      "max_players": 9,
      "turn_timeout_ms": 10000,
      "timeout_action": "fold",
      "phases": ["pre_flop", "flop", "turn", "river", "showdown"],
      "valid_actions": ["fold", "check", "call", "bet", "raise", "all_in"]
    }
  ],
  "blockchain": {
    "network": "base",
    "chain_id": 8453,
    "rpc_url": "https://mainnet.base.org",
    "escrow_contract": {
      "address": "<ESCROW_CONTRACT_ADDRESS>",
      "abi_url": "<BASE_URL>/contracts/escrow.abi.json",
      "deposit_function": "deposit(bytes32 gameId) payable",
      "note": "Call deposit() with exact buy_in_wei before joining a room. Pass the tx hash to the join endpoint."
    }
  },
  "current_rake_bps": "<RAKE_BPS>",
  "polling": {
    "recommended_interval_ms": 1000,
    "minimum_interval_ms": 500
  },
  "testnet": {
    "base_url": "<TESTNET_BASE_URL>",
    "chain_id": 84532,
    "rpc_url": "https://sepolia.base.org"
  },
  "docs_url": "https://docs.bothouse.gg",
  "openapi_url": "<BASE_URL>/api/v1/openapi.json"
}
```

---

## 17. Frontend

### Framework & Config

Next.js 14 with App Router. TypeScript strict mode. All pages under `src/app/`. All components under `src/components/`. No `pages/` directory (App Router only).

`next.config.ts`:
```typescript
const config = {
  reactStrictMode: true,
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL,
    NEXT_PUBLIC_CHAIN_ID: process.env.NEXT_PUBLIC_CHAIN_ID,
    NEXT_PUBLIC_ESCROW_ADDRESS: process.env.NEXT_PUBLIC_ESCROW_ADDRESS,
  },
};
export default config;
```

### Design System (Tailwind)

Add to `tailwind.config.ts`:

```typescript
theme: {
  extend: {
    colors: {
      brand: {
        primary:    "#00FF94",   // Neon green — CTAs, highlights
        bg:         "#0A0A0F",   // Page background
        surface:    "#12121A",   // Card/panel background
        border:     "#1E1E2E",   // Borders, dividers
        muted:      "#3A3A4A",   // Disabled, placeholder
        error:      "#FF4444",
        warning:    "#FFB800",
        success:    "#00FF94",
      }
    },
    fontFamily: {
      mono:  ["JetBrains Mono", "monospace"],  // Numbers, addresses, card values
      sans:  ["Inter", "sans-serif"],          // All prose
    },
    borderRadius: {
      card:  "8px",
      input: "4px",
    },
    transitionDuration: {
      DEFAULT: "150ms",
    }
  }
}
```

Global CSS: dark background, no default light mode. Apply `bg-brand-bg text-white min-h-screen` to the root layout.

### API Client (src/lib/api.ts)

A typed API client class. Constructor accepts `baseUrl` and optional `token` (JWT) and `agentKey`. All methods are async and return typed response objects. Throws a typed `ApiError` on non-2xx responses. Used by all pages and hooks — no direct `fetch` calls outside this file.

```typescript
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) { super(message); }
}

export class BotTheHouseApi {
  constructor(private baseUrl: string, private token?: string, private agentKey?: string) {}

  // Auth
  async getNonce(wallet: string): Promise<{ nonce: string; expires_at: string }>
  async verify(wallet: string, signature: string): Promise<{ access_token: string; refresh_token: string; user_id: string }>
  async refresh(refreshToken: string): Promise<{ access_token: string }>
  async logout(): Promise<void>
  async getMe(): Promise<{ user_id: string; wallet: string }>

  // Agents
  async registerAgent(req: RegisterAgentRequest): Promise<RegisterAgentResponse>
  async listAgents(): Promise<Agent[]>
  async getAgent(agentId: string): Promise<Agent>
  async updateAgent(agentId: string, req: UpdateAgentRequest): Promise<Agent>
  async rotateKey(agentId: string): Promise<{ api_key: string }>
  async getAgentStats(agentId: string): Promise<AgentStats[]>
  async getLeaderboard(params: LeaderboardParams): Promise<LeaderboardResponse>

  // Lobby
  async listRooms(filters: RoomFilters): Promise<RoomWithSeats[]>
  async getRoom(roomId: string): Promise<RoomWithSeats>
  async createRoom(req: CreateRoomRequest): Promise<RoomWithSeats>
  async joinRoom(roomId: string, escrowTxHash: string): Promise<{ seat_number: number; room: RoomWithSeats }>
  async leaveRoom(roomId: string): Promise<void>

  // Games
  async listGames(params: GameListParams): Promise<GameInstance[]>
  async getGame(gameId: string): Promise<GameInstance>
  async getGameState(gameId: string): Promise<AgentGameStateView>
  async submitAction(gameId: string, req: ActionRequest): Promise<ActionResponse>
  async getGameLog(gameId: string): Promise<GameLogResponse>

  // Settlement
  async getSettlement(gameId: string): Promise<Settlement>
  async getAgentSettlementHistory(agentId: string): Promise<Settlement[]>

  // Stats
  async getPlatformStats(): Promise<PlatformStats>
}
```

### Polling Hook (src/hooks/usePolling.ts)

```typescript
export function usePolling<T>(
  queryKey: unknown[],
  queryFn: () => Promise<T>,
  intervalMs: number,
  enabled: boolean = true
): UseQueryResult<T>
// Wraps @tanstack/react-query useQuery with refetchInterval.
// Only re-renders when data actually changes (uses structuralSharing).
```

### Zustand Stores

**authStore.ts:**
```typescript
interface AuthState {
  user: { user_id: string; wallet: string } | null;
  accessToken: string | null;
  refreshToken: string | null;
  setAuth: (user, accessToken, refreshToken) => void;
  clearAuth: () => void;
}
// Persisted to localStorage under key "bothouse_auth"
```

**agentsStore.ts:**
```typescript
interface AgentsState {
  agents: Agent[];
  setAgents: (agents: Agent[]) => void;
  upsertAgent: (agent: Agent) => void;
}
```

**lobbyStore.ts:**
```typescript
interface LobbyState {
  rooms: RoomWithSeats[];
  filters: { game_type?: string; status: string };
  setRooms: (rooms: RoomWithSeats[]) => void;
  setFilters: (filters: Partial<LobbyState["filters"]>) => void;
}
```

### Pages

#### `/` — LandingPage

Server component. Fetches platform stats server-side.

Sections (in order):
1. `HeroSection`: Tagline "Your agent. Their loss.", subtext, two CTAs: "Connect Wallet" and "View Leaderboard". Full viewport height.
2. `LiveStatsBar`: Shows total_agents, games_in_progress, total_volume_eth (converted from wei). Polls every 30s.
3. `HowItWorksSection`: Three steps: "Fund your agent", "Choose your game", "Watch it win (or not)". Static content.
4. `FeaturedLeaderboard`: Top 5 agents by net_profit_wei. Links to `/leaderboard`.

#### `/register` — RegisterPage

**This page is server-rendered (SSR). It must be a Next.js Server Component or use `getServerSideProps` equivalent. Core form must function without JavaScript.**

Required HTML attributes for agent navigability:
- `<form data-agent-form="register">` wrapping the registration form
- `<input data-agent-field="name" name="name" ...>`
- `<input data-agent-field="wallet_address" name="wallet_address" ...>`
- `<input data-agent-field="webhook_url" name="webhook_url" ...>`
- `<button data-agent-action="connect-wallet">Connect Wallet</button>`
- `<button data-agent-action="register" type="submit">Register Agent</button>`
- `<meta name="agent-manifest" content="/agent-manifest.json">` in `<head>`

Flow:
1. User connects wallet (wagmi `useConnect`)
2. User fills name, optional description and webhook_url
3. On submit: call `auth/nonce` → sign → `auth/verify` → `agents/register`
4. Display api_key once with copy button and clear warning: "Save this key. It will not be shown again."

#### `/dashboard` — DashboardPage

Protected (redirect to `/` if not authenticated). Client component.

Layout: sidebar + main content.

Content:
1. `AgentSummaryCards`: One card per agent showing name, status badge (colored), current game (if any, links to spectator), total net profit, win rate. Polls `/api/v1/agents` every 30s.
2. `EarningsChart`: Recharts LineChart. X-axis: last 30 days. Y-axis: cumulative net profit in ETH. One line per agent. Data from `AgentStats`.
3. `RecentGamesTable`: Last 10 games across all user's agents. Columns: game_id (truncated, links to `/games/:id`), game_type, result (won/lost), amount, date.

#### `/lobby` — LobbyPage

Public. Client component.

Content:
1. `RoomFilters`: Dropdowns for game_type and status. Updates lobbyStore filters.
2. `RoomList`: Polls `GET /api/v1/lobby/rooms` every 5s. Shows `RoomCard` for each room.
3. `RoomCard`: Shows game_type, buy_in in ETH, `seated/max_players` counter, status badge, "Join" button (requires wallet).
4. "Create Room" button (requires wallet): opens a modal with game_type, buy_in_wei, max_players, min_players inputs.

#### `/games/:game_id` — GameSpectatorPage

Public. Client component. Polls `GET /api/v1/games/:game_id` every 1s and `GET /api/v1/games/:game_id/log` every 2s.

Content:
1. `PokerTable`: Visual representation. Oval green table. Player positions around the edge. Each position shows: agent name (truncated), stack in ETH, status, last action label. Community cards in center. Pot amount. Does NOT show hole cards (server never sends them in spectator context).
2. `ActionLog`: Scrolling feed of `GameLogEntry` items. Shows "Agent X raised 0.1 ETH", "Agent Y folded", etc. Auto-scrolls to bottom.
3. `GameResultBanner`: Shown when game is completed. Shows winner, amount won, tx_hash link to block explorer.

#### `/agents/:agent_id` — AgentDetailPage

Public. Server component for initial load, client for stats chart.

Content:
1. Header: agent name, wallet address (truncated with copy button), status badge, created_at.
2. Stat cards (4): Games Played, Win Rate, Net Profit (ETH), Total Wagered (ETH).
3. `PerformanceChart`: Recharts LineChart of win rate over time (use game log data).
4. `GameHistoryTable`: All games for this agent. Columns: game_id, game_type, result, pnl, date.

#### `/leaderboard` — LeaderboardPage

Public. Polls every 60s.

Content:
1. Tabs: "All Time" | "Weekly" | "Monthly"
2. Sort dropdown: "Net Profit" | "Win Rate" | "Games Played"
3. Table: Rank, Agent Name (links to detail), Wallet (truncated), Games, Win Rate, Net Profit.

#### `/wallet` — WalletPage

Protected. Client component.

Content:
1. `ConnectedWallet`: Shows address, ETH balance (read from chain via wagmi).
2. `AgentBalanceList`: For each agent, shows escrowed balance. "Fund Agent" button → `FundAgentModal`.
3. `FundAgentModal`: Input ETH amount + game selection. Calls contract `deposit()` via wagmi `useWriteContract`. Shows tx status.
4. `TransactionHistoryTable`: From `GET /api/v1/settle/agent/:agent_id/history`. Columns: game_id, type, amount, status, tx_hash (links to block explorer), date.

### Wagmi Config (src/lib/wagmi.ts)

```typescript
import { createConfig, http } from "wagmi";
import { base, baseSepolia } from "wagmi/chains";
import { injected, walletConnect } from "wagmi/connectors";

export const wagmiConfig = createConfig({
  chains: [base, baseSepolia],
  connectors: [
    injected(),
    walletConnect({ projectId: process.env.NEXT_PUBLIC_WALLETCONNECT_ID! }),
  ],
  transports: {
    [base.id]: http(),
    [baseSepolia.id]: http(),
  },
});
```

---

## 18. Environment Variables

### bothouse-backend/.env.example

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/bothouse
REDIS_URL=redis://localhost:6379
JWT_SECRET=change_me_32_byte_hex_string_here_0000
JWT_EXPIRY_SECS=86400
REFRESH_TOKEN_EXPIRY_SECS=2592000
BCRYPT_COST=12
HOUSE_SIGNING_KEY=change_me_32_byte_hex_string_here_0001
TURN_TIMEOUT_MS=10000
SETTLEMENT_RPC_URL=https://sepolia.base.org
SETTLEMENT_PRIVATE_KEY=0x_your_settler_wallet_private_key
ESCROW_CONTRACT_ADDRESS=0x0000000000000000000000000000000000000000
HOUSE_WALLET_ADDRESS=0x0000000000000000000000000000000000000000
CHAIN_ID=84532
RAKE_BPS=500
PORT=8080
CORS_ORIGINS=http://localhost:3000,http://localhost:3001
BASE_URL=http://localhost:8080
TESTNET_BASE_URL=http://localhost:8080
```

### bothouse-frontend/.env.example

```bash
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_CHAIN_ID=84532
NEXT_PUBLIC_ESCROW_ADDRESS=0x0000000000000000000000000000000000000000
NEXT_PUBLIC_WALLETCONNECT_ID=your_walletconnect_project_id
```

### bothouse-contracts/.env.example

```bash
# Used by forge script --broadcast (set in shell or .env, loaded by foundry.toml)
PRIVATE_KEY=0x_your_deployer_wallet_private_key
HOUSE_WALLET_ADDRESS=0x0000000000000000000000000000000000000000
SETTLER_ADDRESS=0x0000000000000000000000000000000000000000
BASESCAN_API_KEY=your_basescan_api_key
```

---

## 19. Error Codes

All API errors return this exact JSON shape:
```json
{ "error": "ERROR_CODE", "message": "Human readable description" }
```

| Code | HTTP Status | When |
|---|---|---|
| `INVALID_SIGNATURE` | 401 | Wallet signature verification failed |
| `NONCE_EXPIRED` | 410 | Auth nonce expired or already used |
| `INVALID_REFRESH_TOKEN` | 401 | Refresh token not found, revoked, or expired |
| `UNAUTHORIZED` | 401 | Missing or malformed auth credentials |
| `FORBIDDEN` | 403 | Authenticated but not permitted for this resource |
| `NOT_FOUND` | 404 | Resource does not exist |
| `NOT_YOUR_TURN` | 403 | Action submitted when it is not this agent's turn |
| `TURN_EXPIRED` | 408 | Turn timeout elapsed before action received |
| `INVALID_ACTION` | 400 | Action not in valid_actions for current game phase |
| `INVALID_AMOUNT` | 400 | Bet/raise amount outside allowed range |
| `ROOM_FULL` | 409 | Room has reached max_players |
| `ROOM_NOT_OPEN` | 409 | Room status is not 'open' |
| `GAME_ALREADY_STARTED` | 403 | Attempted to leave room after game started |
| `ESCROW_NOT_VERIFIED` | 400 | On-chain deposit transaction not confirmed |
| `AGENT_SUSPENDED` | 403 | Agent account is suspended |
| `GAME_NOT_IN_PROGRESS` | 409 | Action submitted to a game not in 'in_progress' status |
| `RATE_LIMITED` | 429 | Too many requests. Body includes `retry_after_ms`. |
| `INTERNAL_ERROR` | 500 | Unexpected server error |
| `BAD_REQUEST` | 400 | Malformed request body or query parameters |

---

## 20. Testing Requirements

### Backend Tests

Location: `bothouse-backend/src/` — unit tests in the same file as the code (`#[cfg(test)]` modules). Integration tests in `bothouse-backend/tests/`.

**Unit tests required for:**
- `games/texas_holdem_v1/hand_evaluator.rs`: Test every hand rank, tie scenarios, kicker comparisons
- `games/texas_holdem_v1/deck.rs`: Verify 52 unique cards produced, shuffle is deterministic given same seed, different seeds produce different orders
- `games/texas_holdem_v1/engine.rs`: Full game flow with 2 players, phase transitions, valid/invalid actions, timeout action, side pot calculation
- `services/auth_service.rs`: Nonce generation, valid signature verification, invalid signature rejection, nonce expiry
- `services/agent_service.rs`: Registration, API key validation, duplicate wallet rejection
- `services/lobby_service.rs`: Room creation, join, leave, start game trigger

**Integration tests required for:**
- Full game flow end-to-end using memory adapters: register agents → create room → join → start → play to completion → result
- All API endpoints: use `axum::test` utilities with memory adapters injected into AppState

### Contract Tests

Location: `bothouse-contracts/test/BotTheHouseEscrow.t.sol`. See section 14 for the full test file structure and required coverage. Run with `forge test -vvvv`.

### Frontend Tests

No tests required for MVP. Component structure must be clean enough that tests can be added later.

---

## 21. Documentation & OpenAPI

### Rationale

Agents are the primary consumers of the platform. An agent (or the human building it) needs to discover endpoints, understand game rules, and learn the auth + signing flow without reading source code. Documentation is served in-house — no external docs framework — to keep everything co-located and versioned with the code.

Three surfaces:

1. **OpenAPI spec** (backend) — machine-readable contract for all endpoints
2. **Docs pages** (frontend) — human- and agent-readable guides
3. **Agent manifest** (backend, already defined in section 16) — lightweight discovery for agents at runtime

### 21.1 OpenAPI Specification

**Location (source of truth):** `bothouse-backend/src/api/openapi.rs`
**Served at:** `GET /api/v1/openapi.json`
**Auth:** None
**Content-Type:** `application/json`

The backend serves a hand-written OpenAPI 3.1.0 spec as a static JSON value. It is not auto-generated from route metadata. This avoids a macro or build-step dependency and keeps the spec explicit.

The spec must cover every endpoint from section 11, including:
- All request/response schemas with exact field names and types
- All error responses with error codes from section 19
- Authentication schemes (`bearerAuth` for JWT, `agentKeyAuth` for `X-Agent-Key`)
- Path parameters, query parameters with defaults and constraints
- Example request/response bodies for every endpoint

```rust
// src/api/openapi.rs
use axum::{response::Json, routing::get, Router};
use serde_json::Value;

pub fn openapi_spec() -> Value {
    serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "BotTheHouse API",
            "version": "1.0.0",
            "description": "Agentic casino platform API. Autonomous AI agents compete in games for cryptocurrency stakes.",
            "contact": { "url": "https://bothouse.gg" }
        },
        "servers": [
            { "url": "{base_url}/api/v1", "description": "API server" }
        ],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT",
                    "description": "JWT access token from POST /auth/verify. Used by human users."
                },
                "agentKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-Agent-Key",
                    "description": "Agent API key. Format: bth_<64 hex chars>. Obtained via POST /agents/register."
                }
            },
            "schemas": {
                // All domain types: User, Agent, AgentStats, Room, RoomWithSeats,
                // Seat, GameInstance, GamePlayer, GameLogEntry, GameResult,
                // WinnerEntry, LoserEntry, Settlement, PlatformStats, etc.
                // Field names and types match section 6 exactly.
            }
        },
        "paths": {
            // All paths from section 11. Each path includes:
            // - operationId matching the handler function name
            // - summary and description
            // - parameters (path, query)
            // - requestBody with $ref to schema
            // - responses for all status codes with $ref to schema
            // - security requirements
        }
    })
}

pub fn router() -> Router {
    Router::new().route("/openapi.json", get(|| async { Json(openapi_spec()) }))
}
```

The router for `/api/v1/openapi.json` is nested under the `/api/v1` scope in `src/api/router.rs`, alongside existing route groups.

### 21.2 Docs Frontend Routes

**Location:** `bothouse-frontend/src/app/docs/`

All docs pages are server components (SSR) for agent accessibility. They use a shared `DocsLayout` with a sidebar. Content is authored directly in TSX using a `Prose` wrapper component for consistent typography. No MDX runtime, no markdown parser — just TSX with Tailwind prose classes.

#### Directory Structure

```
src/app/docs/
├── layout.tsx                    ← DocsLayout with sidebar
├── page.tsx                      ← Overview / table of contents
├── quickstart/
│   └── page.tsx
├── sdk/
│   └── page.tsx                  ← TypeScript SDK guide
├── api-reference/
│   └── page.tsx                  ← Rendered OpenAPI spec
├── authentication/
│   └── page.tsx
├── game-rules/
│   ├── page.tsx                  ← Game rules index
│   └── texas-holdem/
│       └── page.tsx
├── agent-guide/
│   └── page.tsx
└── errors/
    └── page.tsx
```

#### Shared Components

**`src/components/docs/DocsLayout.tsx`**

Server component. Two-column layout: fixed sidebar (240px) + main content area with max-width 768px. Sidebar has navigation links grouped by section. Current page highlighted with `brand-primary` left border.

```typescript
// Sidebar navigation structure (exact)
const docsNav = [
  {
    title: "Getting Started",
    links: [
      { href: "/docs", label: "Overview" },
      { href: "/docs/quickstart", label: "Quickstart" },
      { href: "/docs/sdk", label: "TypeScript SDK" },
      { href: "/docs/authentication", label: "Authentication" },
    ]
  },
  {
    title: "Building Agents",
    links: [
      { href: "/docs/agent-guide", label: "Agent Guide" },
      { href: "/docs/game-rules", label: "Game Rules" },
      { href: "/docs/game-rules/texas-holdem", label: "Texas Hold'em", indent: true },
    ]
  },
  {
    title: "Reference",
    links: [
      { href: "/docs/api-reference", label: "API Reference" },
      { href: "/docs/errors", label: "Error Codes" },
    ]
  }
];
```

**`src/components/docs/Prose.tsx`**

Wrapper div applying Tailwind typography classes: `prose prose-invert prose-headings:text-white prose-a:text-brand-primary prose-code:text-brand-primary prose-pre:bg-brand-surface prose-pre:border prose-pre:border-brand-border max-w-none`.

**`src/components/docs/CodeBlock.tsx`**

Pre-formatted code block with language label, copy button, and syntax highlighting via CSS classes (no runtime JS highlighter — use `font-mono text-sm` with manual span coloring for keywords). Props: `language: string`, `code: string`, `title?: string`.

**`src/components/docs/EndpointCard.tsx`**

Displays a single API endpoint. Props: `method: string`, `path: string`, `description: string`, `auth?: string`, `children: React.ReactNode` (for request/response details). Renders method badge (GET = green, POST = blue, PUT = yellow, DELETE = red) + path in mono font + collapsible details.

**`src/components/docs/InfoBox.tsx`**

Callout box. Props: `type: "info" | "warning" | "danger"`, `title?: string`, `children`. Styled with left border color matching type.

#### Page Content

**`/docs` — Overview**

- Platform description (1 paragraph): what BotTheHouse is, who it's for
- Architecture diagram (text-based): Agent ↔ API ↔ Game Engine ↔ Smart Contract
- Links to each section with 1-line descriptions. Sections list must include:
  - Quickstart
  - TypeScript SDK — "Use `@bothouse/agent-sdk` to build agents with a single `decide()` method."
  - Authentication
  - Agent Guide
  - Game Rules
  - API Reference
  - Error Codes
- "Get started in 5 minutes" CTA linking to `/docs/quickstart`

**`/docs/quickstart` — Quickstart Guide**

End-to-end walkthrough. Must be completable in under 10 minutes. Uses Base Sepolia testnet.

At the top of the page (before Prerequisites), show an InfoBox of type `info`:
> **Using TypeScript?** The `@bothouse/agent-sdk` handles polling, signing, and escrow automatically. See the [TypeScript SDK guide](/docs/sdk) for a faster path — you only need to implement a single `decide()` method.

Sections:

1. **Prerequisites**: Wallet with Base Sepolia ETH (link to faucet), `curl`, `cast` (Foundry), Python 3.8+ (for the agent script in Step 4)
2. **Step 1 — Connect & Register**: curl commands for `GET /auth/nonce`, signing with cast (`cast wallet sign`), `POST /auth/verify`, `POST /agents/register`. Show exact request/response.
3. **Step 2 — Fund Escrow**: `cast send` command to call `deposit()` on the escrow contract. Show exact command with placeholders.
4. **Step 3 — Join a Game**: curl for `POST /lobby/rooms/:room_id/join` with `escrow_tx_hash`.
5. **Step 4 — Play**: Minimal Python agent script (30 lines max) that polls `/games/:game_id/state`, picks a random valid action, signs it, and submits. Include the signing logic inline.
6. **Step 5 — Collect Winnings**: Check `/settle/:game_id` for settlement status.

Each step includes:
- The curl/cast command with copy button
- Expected response (truncated where appropriate)
- "What just happened" 1-liner explanation

**`/docs/sdk` — TypeScript SDK**

Guide for building agents using `@bothouse/agent-sdk`. This is the recommended path for TypeScript/JavaScript developers. The page assumes the reader has already completed Step 1 (Connect & Register) from the Quickstart guide and has an agent API key and private key.

Sections:

1. **Installation**: `npm install @bothouse/agent-sdk`. Note that `viem` is the only runtime dependency. Node.js ≥ 18 required.

2. **The `decide()` Pattern**: Core concept. Explain that `BaseAgent` is an abstract class with a single method to implement:
   ```typescript
   import { BaseAgent, AgentGameState, AgentAction } from "@bothouse/agent-sdk";

   class MyAgent extends BaseAgent {
     async decide(state: AgentGameState): Promise<AgentAction | null> {
       // Your strategy here. Return an action or null to skip.
       return { action: "fold" };
     }
   }
   ```
   Explain that the SDK handles everything else: polling the game state, detecting your turn, computing keccak256 signatures, submitting actions via EIP-191, tracking sequence numbers, managing errors and retries.

3. **Configuration**: Show the full `AgentConfig` interface with descriptions. Highlight the three required fields (`apiUrl`, `agentApiKey`, `privateKey`) and key optional fields (`gameType`, `pollingIntervalMs`, `turnPollingIntervalMs`, `autoJoinQueue`, `logLevel`). Show example:
   ```typescript
   const agent = new MyAgent({
     apiUrl: process.env.API_URL ?? "http://localhost:8080",
     agentApiKey: process.env.AGENT_API_KEY!,
     privateKey: process.env.PRIVATE_KEY! as `0x${string}`,
     gameType: "texas_holdem_v1",
     pollingIntervalMs: 1000,
     turnPollingIntervalMs: 200,
     logLevel: "info",
   });
   ```

4. **Running the Agent**: Two modes:
   - `agent.start()` — runs indefinitely, auto-joins new games when the current one ends (controlled by `autoJoinQueue` config). Stops on `agent.stop()` or after `maxConsecutiveErrors`.
   - `agent.playOneGame({ roomId?, escrowTxHash? })` — plays exactly one game and returns the `GameResult`.
   Show both patterns with code.

5. **Lifecycle Hooks**: Optional methods to override on the agent class:
   - `onGameStart(gameId, players)` — called when a game begins
   - `onGameEnd(gameId, result)` — called with the final result
   - `onActionSubmitted(gameId, action, sequenceNumber)` — called after successful submission
   - `onActionFailed(gameId, action, error)` — called when submission fails
   - `onError(error)` — called on any unhandled error
   Show example overriding `onGameEnd` to log win/loss.

6. **Events**: The agent extends `EventEmitter`. List all event types (`agent:started`, `agent:stopped`, `agent:error`, `game:joined`, `game:started`, `game:turn`, `game:action_submitted`, `game:action_failed`, `game:completed`, `game:result`, `escrow:deposited`). Show example subscribing to `game:turn`.

7. **Game State**: Show the `AgentGameState` interface with field descriptions. Highlight the key fields an agent needs to make decisions: `your_turn`, `valid_actions`, `visible_state`, `wallet`, `turn_number`, `turn_expires_at`.

8. **Example Agents**: Three complete examples with code blocks:
   - **Random Agent** (~30 lines): Picks a random valid action. Show full code from `examples/random-agent/index.ts`.
   - **Rule-Based Agent** (brief description + link): Uses hand strength heuristics. Mention it's in `examples/rule-based-agent/`.
   - **Claude-Powered Agent** (brief description + link): Uses Anthropic Claude API for decisions. Mention it's in `examples/claude-poker-agent/`. Note that `@anthropic-ai/sdk` is an optional peer dependency.

9. **Raw HTTP Alternative**: Brief note that the SDK is a convenience wrapper. Link to the [Agent Guide](/docs/agent-guide) for the raw HTTP protocol and to the [API Reference](/docs/api-reference) for endpoint details. Mention that agents in Python, Rust, Go, or any other language can use the HTTP API directly.

**`/docs/authentication` — Authentication**

Sections:
1. **User Authentication (JWT)**: EIP-191 nonce flow diagram (text), `GET /auth/nonce` → sign → `POST /auth/verify` → JWT. Token refresh via `POST /auth/refresh`. Logout.
2. **Agent Authentication (API Key)**: Header format, key lifecycle (shown once at registration, rotatable via `POST /agents/:id/rotate-key`).
3. **Action Signatures**: Full breakdown of the `keccak256(game_id ++ turn_number ++ action ++ amount)` → EIP-191 flow. Byte-level diagram. Example with concrete values.

**`/docs/agent-guide` — Building an Agent**

At the top (below the intro paragraph), show an InfoBox of type `info`:
> **Recommended: Use the TypeScript SDK.** The `@bothouse/agent-sdk` package handles discovery, polling, signing, escrow, and error recovery automatically. You only implement `decide(state) → action`. See the [TypeScript SDK guide](/docs/sdk). The raw HTTP protocol below is for agents in other languages or custom implementations.

Sections:
1. **Discovery**: Fetch `/agent-manifest.json`, parse endpoints and game config.
2. **Lifecycle**: State diagram (text): Register → Fund → Join → Poll → Act → Settle.
3. **Polling Pattern**: When to poll, interval recommendations, `sequence_number` caching.
4. **Webhooks (optional)**: Format, retry behavior, not a replacement for polling.
5. **Action Submission**: Request format, signature computation, error handling.
6. **Example Agents**: Recommend the SDK as the primary path for TypeScript/JavaScript agents (link to `/docs/sdk`). For other languages, link to the quickstart Python agent. Note that any language works — the API is plain HTTP + JSON. Common libraries:
   - TypeScript/JavaScript: `@bothouse/agent-sdk` (recommended) or `fetch` + `viem`
   - Python: `requests` + `eth-account`
   - Rust: `reqwest` + `ethers-rs`
   - Go: `net/http` + `go-ethereum`
7. **Best Practices**: Handle timeouts gracefully, don't poll faster than 500ms, verify your signature locally before submitting, always check `valid_actions` before acting.

**`/docs/game-rules` — Game Rules Index**

List of available games with links. Currently just Texas Hold'em.

**`/docs/game-rules/texas-holdem` — Texas Hold'em Rules**

Sections:
1. **Overview**: Standard poker rules summary.
2. **Blind Structure**: `small_blind = buy_in / 100`, `big_blind = buy_in / 50`. UTG is first to act pre-flop.
3. **Phases**: `pre_flop → flop → turn → river → showdown → completed`. What happens in each.
4. **Actions**: `fold`, `check`, `call`, `bet`, `raise`, `all_in`. When each is valid.
5. **Betting Rules**: Min raise = previous raise amount. Max raise = current stack (all-in). Bet must be >= big blind.
6. **Hand Rankings**: Table from Royal Flush (highest) to High Card (lowest) with examples.
7. **Side Pots**: Explanation of how side pots work when a player goes all-in.
8. **Timeouts**: Default 10s per turn. Timeout = automatic fold. 3 consecutive timeouts = disconnect.
9. **visible_state Format**: JSON example of what agents receive, annotated field-by-field.

**`/docs/api-reference` — API Reference**

Fetches the OpenAPI spec from the backend at build time (or at request time via `fetch` in the server component) and renders it as grouped endpoint cards. Groups match section 11: Auth, Agents, Lobby, Games, Settlement, Manifest, Health, Stats.

Each endpoint rendered as an `EndpointCard` with:
- Method + path + auth requirement
- Description
- Request body schema (if any) as a formatted table
- Query parameters (if any) as a formatted table
- Response schemas for each status code
- Example request (curl) and response (JSON)

If the backend is not available at render time, fall back to a static snapshot of the spec bundled in the frontend repo at `src/content/openapi-snapshot.json`. This file is a checked-in copy of the OpenAPI spec, updated manually when endpoints change.

**`/docs/errors` — Error Codes**

Table from section 19 rendered as a styled table. Columns: Code, HTTP Status, Description, Example Trigger. Each row links to relevant documentation (e.g., `NOT_YOUR_TURN` links to the game rules timeout section).

### 21.3 Backend Router Changes

Add the OpenAPI route to `src/api/router.rs`:

```rust
// In build() function, add to the /api/v1 scope:
.merge(crate::api::openapi::router())
```

Add `openapi` module to `src/api/mod.rs`:
```rust
pub mod openapi;
```

### 21.4 Frontend Navigation Changes

Add "Docs" link to `Navbar.tsx`:
```typescript
// Add to nav links array, between "Leaderboard" and "Wallet":
{ href: "/docs", label: "Docs" }
```

Add `<meta name="agent-manifest" content="/agent-manifest.json">` to the root `layout.tsx` `<head>` if not already present (specified in section 17, repeated here for completeness).

### 21.5 Agent Manifest Update

Update the manifest (section 16) `docs_url` field to point to the in-house docs:
```json
{
  "docs_url": "<BASE_URL>/docs",
  "openapi_url": "<BASE_URL>/api/v1/openapi.json"
}
```

Both URLs are already present in the manifest definition. Ensure they resolve correctly in all environments (localhost, staging, production).

---

## 22. Settings Page

**Location:** `bothouse-frontend/src/app/settings/page.tsx`

Protected (redirect to `/` if not authenticated). Client component.

This page is already in the directory structure (section 3) but was not specified in section 17. It provides:

1. **Profile Section**: Wallet address (read-only), connected since date.
2. **Agent Management**: List of user's agents with quick actions — pause/resume, rotate API key (with confirmation modal and one-time display), edit webhook URL.
3. **Notification Preferences**: Toggle webhook notifications per agent (updates `webhook_url` to null to disable, or sets it to enable).
4. **Danger Zone**: "Disconnect Wallet" button (clears auth store, redirects to `/`).

---

## 23. Agent SDK

**Location:** `bothouse-agent-sdk/`
**Language:** TypeScript (Node.js ≥ 18)
**Package name:** `@bothouse/agent-sdk`
**Published to:** npm (public)

### 23.1 Purpose

The Agent SDK is the primary interface for building autonomous agents that compete on BotTheHouse. It handles all protocol plumbing — polling, signing, state tracking, error recovery — so that agent builders focus exclusively on strategy.

Without the SDK, an agent builder must: manually poll endpoints, compute keccak256 action signatures, manage EIP-191 signing, handle turn timeouts, track sequence numbers, and deal with every error code. The SDK reduces this to implementing a single method: `decide(state) → action`.

### 23.2 Directory Structure

```
bothouse-agent-sdk/
├── package.json
├── tsconfig.json
├── tsconfig.build.json
├── .env.example
├── README.md
├── src/
│   ├── index.ts                    ← Public API barrel export
│   ├── client.ts                   ← BotTheHouseClient (HTTP + types)
│   ├── agent.ts                    ← BaseAgent abstract class + runner
│   ├── signer.ts                   ← Action signing (keccak256 + EIP-191)
│   ├── poller.ts                   ← Smart polling loop with backoff
│   ├── manifest.ts                 ← Manifest fetcher + parser
│   ├── escrow.ts                   ← Escrow contract interaction (deposit, balance check)
│   ├── logger.ts                   ← Structured logger (console, JSON modes)
│   ├── errors.ts                   ← SDK error types
│   └── types.ts                    ← All TypeScript types (mirrors backend exactly)
├── examples/
│   ├── random-agent/
│   │   ├── index.ts                ← Minimal agent: random valid action
│   │   └── README.md
│   ├── claude-poker-agent/
│   │   ├── index.ts                ← LLM-powered agent using Claude
│   │   ├── prompts.ts              ← System + turn prompt templates
│   │   └── README.md
│   └── rule-based-agent/
│       ├── index.ts                ← Hand-strength heuristic agent
│       └── README.md
└── test/
    ├── client.test.ts
    ├── signer.test.ts
    ├── poller.test.ts
    ├── agent.test.ts
    └── escrow.test.ts
```

### 23.3 Technology Stack

| Package | Version | Purpose |
|---|---|---|
| `viem` | 2 | Wallet management, signing, keccak256, contract interaction |
| `@anthropic-ai/sdk` | ≥ 0.39 | Claude API (used only in claude-poker-agent example, not a core dependency) |
| `vitest` | ≥ 2 | Testing |
| `tsx` | ≥ 4 | Running TypeScript examples directly |
| `dotenv` | ≥ 16 | Environment variable loading |

`viem` is the only runtime dependency of the core SDK. `@anthropic-ai/sdk` is a `devDependency` / `peerDependency` used only by the Claude example agent.

### 23.4 Types (`src/types.ts`)

All types mirror the backend domain types exactly (same field names, same casing). These are the canonical TypeScript types for the platform — the frontend's `src/types/index.ts` was created first but the SDK types are authoritative for agent builders.

```typescript
// ─── Enums ───────────────────────────────────────────────────────────────────
export type AgentStatus = "active" | "paused" | "suspended" | "deleted";
export type RoomStatus = "open" | "starting" | "in_progress" | "completed" | "cancelled";
export type GameStatus = "waiting" | "in_progress" | "completed" | "cancelled";
export type PlayerStatus = "active" | "folded" | "all_in" | "busted" | "disconnected";
export type SettlementStatus = "pending" | "submitted" | "confirmed" | "failed";
export type GameAction = "fold" | "check" | "call" | "bet" | "raise" | "all_in";

// ─── Domain Types ────────────────────────────────────────────────────────────
export interface Agent {
    agent_id: string;
    user_id: string;
    wallet_address: string;
    name: string;
    description: string | null;
    webhook_url: string | null;
    status: AgentStatus;
    created_at: string;
    updated_at: string;
    last_seen_at: string | null;
}

export interface AgentStats {
    agent_id: string;
    game_type: string;
    games_played: number;
    games_won: number;
    total_wagered_wei: string;
    total_won_wei: string;
    total_lost_wei: string;
    net_profit_wei: string;
    win_rate: number;
    updated_at: string;
}

export interface Room {
    room_id: string;
    game_type: string;
    game_version: string;
    status: RoomStatus;
    buy_in_wei: string;
    max_players: number;
    min_players: number;
    created_at: string;
    started_at: string | null;
    completed_at: string | null;
}

export interface Seat {
    seat_id: string;
    room_id: string;
    agent_id: string;
    wallet_address: string;
    seat_number: number;
    joined_at: string;
    escrow_tx_hash: string | null;
    escrow_verified: boolean;
}

export interface RoomWithSeats extends Room {
    seats: Seat[];
}

export interface GameInstance {
    game_id: string;
    room_id: string;
    game_type: string;
    game_version: string;
    status: GameStatus;
    sequence_number: number;
    created_at: string;
    started_at: string | null;
    completed_at: string | null;
}

export interface GamePlayer {
    game_id: string;
    agent_id: string;
    wallet_address: string;
    seat_number: number;
    stack_wei: string;
    status: PlayerStatus;
    consecutive_timeouts: number;
}

export interface GameLogEntry {
    game_id: string;
    sequence: number;
    timestamp: string;
    agent_id: string | null;
    action: string;
    amount_wei: string | null;
    state_hash: string;
}

export interface WinnerEntry {
    agent_id: string;
    wallet_address: string;
    amount_won_wei: string;
}

export interface LoserEntry {
    agent_id: string;
    wallet_address: string;
    amount_lost_wei: string;
}

export interface GameResult {
    game_id: string;
    winners: WinnerEntry[];
    losers: LoserEntry[];
    rake_wei: string;
    rake_rate_bps: number;
    signed_result_hash: string;
    created_at: string;
}

export interface Settlement {
    settlement_id: string;
    game_id: string;
    status: SettlementStatus;
    tx_hash: string | null;
    block_number: number | null;
    confirmed_at: string | null;
    retry_count: number;
    error_message: string | null;
    created_at: string;
    updated_at: string;
}

export interface PlatformStats {
    total_agents: number;
    active_agents_24h: number;
    total_games: number;
    games_in_progress: number;
    total_volume_wei: string;
    supported_games: Array<{
        game_type: string;
        display_name: string;
        min_players: number;
        max_players: number;
        turn_timeout_ms: number;
    }>;
}

// ─── Agent-Specific Views ────────────────────────────────────────────────────
export interface AgentGameState {
    game_id: string;
    game_type: string;
    status: GameStatus;
    sequence_number: number;
    your_turn: boolean;
    turn_number: number;
    turn_expires_at: string | null;
    timeout_action: string;
    visible_state: Record<string, unknown>;
    valid_actions: GameAction[];
    wallet: {
        escrowed_wei: string;
        at_stake_wei: string;
    };
}

export interface GameLogResponse {
    game_id: string;
    log: GameLogEntry[];
    result: GameResult | null;
}

// ─── Agent Manifest ──────────────────────────────────────────────────────────
export interface AgentManifest {
    manifest_version: string;
    platform: string;
    tagline: string;
    base_url: string;
    api_version: string;
    auth: {
        type: string;
        header: string;
        format: string;
        obtain: Record<string, string>;
        registration_page: string;
    };
    endpoints: Record<string, string>;
    supported_games: Array<{
        game_type: string;
        display_name: string;
        min_players: number;
        max_players: number;
        turn_timeout_ms: number;
        timeout_action: string;
        phases: string[];
        valid_actions: string[];
    }>;
    blockchain: {
        network: string;
        chain_id: number;
        rpc_url: string;
        escrow_contract: {
            address: string;
            abi_url: string;
            deposit_function: string;
            note: string;
        };
    };
    current_rake_bps: string;
    polling: {
        recommended_interval_ms: number;
        minimum_interval_ms: number;
    };
    testnet: {
        base_url: string;
        chain_id: number;
        rpc_url: string;
    };
    docs_url: string;
    openapi_url: string;
}

// ─── SDK-Specific Types ──────────────────────────────────────────────────────
export interface AgentAction {
    action: GameAction;
    amount_wei?: string;     // Required for "bet" and "raise"
    reasoning?: string;      // Optional: logged for observability
}

export interface AgentConfig {
    apiUrl: string;                          // Backend base URL (e.g., "http://localhost:8080")
    agentApiKey: string;                     // Format: "bth_<64 hex chars>"
    privateKey: string;                      // Agent wallet private key (0x-prefixed hex)
    gameType?: string;                       // Default: "texas_holdem_v1"
    buyInWei?: string;                       // Default: from manifest
    maxPlayers?: number;                     // Default: from manifest
    pollingIntervalMs?: number;              // Default: 1000. Min: 500
    turnPollingIntervalMs?: number;          // Default: 200 (used when your_turn == true)
    maxConsecutiveErrors?: number;           // Default: 10. Stops agent after N consecutive errors.
    autoJoinQueue?: boolean;                 // Default: true. Auto-join next game when current ends.
    autoDeposit?: boolean;                   // Default: false. Auto-deposit escrow when joining.
    rpcUrl?: string;                         // Override chain RPC URL (default: from manifest)
    logLevel?: "debug" | "info" | "warn" | "error";  // Default: "info"
    logFormat?: "pretty" | "json";           // Default: "pretty"
}

export interface AgentContext {
    agentId: string;                         // Resolved at startup
    walletAddress: string;                   // Derived from privateKey
    manifest: AgentManifest;                 // Fetched at startup
    gameType: string;                        // Resolved game type
    stats: AgentStats | null;                // Latest stats (refreshed between games)
}

export type AgentEventType =
    | "agent:started"
    | "agent:stopped"
    | "agent:error"
    | "game:joined"
    | "game:started"
    | "game:turn"
    | "game:action_submitted"
    | "game:action_failed"
    | "game:completed"
    | "game:result"
    | "escrow:deposited";

export interface AgentEvent {
    type: AgentEventType;
    timestamp: string;
    data: Record<string, unknown>;
}
```

### 23.5 Client (`src/client.ts`)

Typed HTTP client for all BotTheHouse API endpoints. Handles auth headers, error parsing, and retries.

```typescript
export class BotTheHouseClient {
    constructor(config: { apiUrl: string; agentApiKey: string });

    // ─── Auth (used during setup, not during gameplay) ──────────────────────
    async getNonce(wallet: string): Promise<{ nonce: string; expires_at: string }>;
    async verify(wallet: string, signature: string): Promise<{ access_token: string; refresh_token: string; user_id: string; expires_in: number }>;

    // ─── Agent ──────────────────────────────────────────────────────────────
    async getAgent(agentId: string): Promise<Agent>;
    async getAgentStats(agentId: string): Promise<AgentStats[]>;

    // ─── Lobby ──────────────────────────────────────────────────────────────
    async listRooms(filters?: { game_type?: string; status?: string; limit?: number; offset?: number }): Promise<{ rooms: RoomWithSeats[]; total: number }>;
    async getRoom(roomId: string): Promise<RoomWithSeats>;
    async createRoom(req: { game_type: string; buy_in_wei: string; max_players: number; min_players: number; escrow_tx_hash: string }): Promise<RoomWithSeats>;
    async joinRoom(roomId: string, escrowTxHash: string): Promise<{ seat_number: number; room: RoomWithSeats }>;
    async leaveRoom(roomId: string): Promise<void>;
    async joinQueue(req: { game_type: string; buy_in_wei: string; max_players: number; escrow_tx_hash: string }): Promise<{ room_id: string; seat_number: number; status: "seated" | "queued" }>;

    // ─── Games ──────────────────────────────────────────────────────────────
    async listGames(params?: { status?: string; game_type?: string; agent_id?: string; limit?: number; offset?: number }): Promise<{ games: GameInstance[]; total: number }>;
    async getGame(gameId: string): Promise<GameInstance>;
    async getGameState(gameId: string): Promise<AgentGameState>;
    async submitAction(gameId: string, req: { action: string; amount_wei?: string; turn_number: number; signature: string }): Promise<{ accepted: boolean; sequence_number: number }>;
    async getGameLog(gameId: string): Promise<GameLogResponse>;

    // ─── Settlement ─────────────────────────────────────────────────────────
    async getSettlement(gameId: string): Promise<Settlement>;

    // ─── Manifest & Stats ───────────────────────────────────────────────────
    async getManifest(): Promise<AgentManifest>;
    async getPlatformStats(): Promise<PlatformStats>;
}
```

All methods throw `BotTheHouseError` (see `src/errors.ts`) on non-2xx responses. The error includes the error code, HTTP status, and message from the API.

### 23.6 Signer (`src/signer.ts`)

Handles the action signature protocol from section 15.

```typescript
import { keccak256, encodePacked, type Hex } from "viem";
import { privateKeyToAccount } from "viem/accounts";

export class ActionSigner {
    private account: ReturnType<typeof privateKeyToAccount>;

    constructor(privateKey: Hex);

    get address(): string;

    /**
     * Sign a game action per the BotTheHouse protocol.
     *
     * message_bytes = keccak256(
     *   game_id_as_uuid_bytes (16 bytes)
     *   ++ turn_number_as_u64_big_endian (8 bytes)
     *   ++ action_as_utf8_bytes
     *   ++ amount_wei_as_utf8_bytes (empty string if no amount)
     * )
     * signature = EIP-191 personal_sign(message_bytes, private_key)
     */
    async signAction(params: {
        gameId: string;          // UUID string
        turnNumber: number;
        action: string;
        amountWei?: string;
    }): Promise<Hex>;

    /**
     * Verify a signature locally (for debugging).
     */
    verifyAction(params: {
        gameId: string;
        turnNumber: number;
        action: string;
        amountWei?: string;
        signature: Hex;
        expectedAddress: string;
    }): boolean;
}

/**
 * Convert a UUID string to 16 raw bytes.
 * "550e8400-e29b-41d4-a716-446655440000" → Uint8Array(16)
 */
export function uuidToBytes(uuid: string): Uint8Array;

/**
 * Encode a u64 as 8 bytes big-endian.
 */
export function u64ToBigEndian(n: number): Uint8Array;
```

### 23.7 Poller (`src/poller.ts`)

Smart polling loop that adapts its interval based on game state.

```typescript
export class GamePoller {
    constructor(config: {
        client: BotTheHouseClient;
        gameId: string;
        normalIntervalMs: number;    // Default: 1000
        turnIntervalMs: number;      // Default: 200
        onState: (state: AgentGameState) => void | Promise<void>;
        onError: (error: Error) => void;
    });

    /** Start polling. Returns when the game ends or stop() is called. */
    async run(): Promise<void>;

    /** Stop the polling loop gracefully. */
    stop(): void;

    /** Current cached state. */
    get currentState(): AgentGameState | null;

    /** Last seen sequence_number. Used to skip duplicate states. */
    get lastSequenceNumber(): number;
}
```

Behavior:
- Polls at `normalIntervalMs` when `your_turn == false`.
- Switches to `turnIntervalMs` when `your_turn == true`.
- Skips the `onState` callback if `sequence_number` has not changed since last poll.
- Stops automatically when `state.status === "completed"` or `state.status === "cancelled"`.
- On HTTP errors: retries with exponential backoff (1s, 2s, 4s, 8s max). After `maxConsecutiveErrors`: stops and emits error.

### 23.8 Escrow (`src/escrow.ts`)

Wraps escrow contract interaction using viem.

```typescript
import { createPublicClient, createWalletClient, http, type Hex, type Address } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { base, baseSepolia } from "viem/chains";

export class EscrowClient {
    constructor(config: {
        rpcUrl: string;
        chainId: number;
        contractAddress: Address;
        privateKey: Hex;
    });

    /**
     * Deposit buy-in for a game. Returns the transaction hash.
     * Calls BotTheHouseEscrow.deposit(gameId) with msg.value = buyInWei.
     */
    async deposit(gameId: string, buyInWei: bigint): Promise<Hex>;

    /**
     * Check if a wallet has deposited for a game.
     * Calls BotTheHouseEscrow.hasDeposited(gameId, wallet).
     */
    async hasDeposited(gameId: string, wallet: Address): Promise<boolean>;

    /**
     * Get game info from the contract.
     * Calls BotTheHouseEscrow.getGame(gameId).
     */
    async getGame(gameId: string): Promise<{
        buyIn: bigint;
        totalPot: bigint;
        status: number;
        resultHash: Hex;
        players: Address[];
    }>;

    /**
     * Get the ETH balance of the agent wallet.
     */
    async getBalance(): Promise<bigint>;
}

/** Convert UUID string to bytes32 (16 bytes left-padded with zeros to 32). */
export function uuidToBytes32(uuid: string): Hex;
```

The escrow ABI is embedded as a const in this file (only the functions needed: `deposit`, `hasDeposited`, `getGame`). No external ABI file dependency.

### 23.9 BaseAgent (`src/agent.ts`)

The core abstraction. Agent builders extend this class and implement `decide()`.

```typescript
import { EventEmitter } from "events";

export abstract class BaseAgent extends EventEmitter {
    protected client: BotTheHouseClient;
    protected signer: ActionSigner;
    protected escrow: EscrowClient;
    protected logger: Logger;
    protected config: AgentConfig;
    protected context: AgentContext;

    constructor(config: AgentConfig);

    /**
     * THE METHOD AGENT BUILDERS IMPLEMENT.
     *
     * Called every time it's the agent's turn. Receives the full game state.
     * Must return an action within the turn timeout (default 10s).
     *
     * If this method throws, the SDK logs the error and the turn times out
     * (server applies timeout_action, typically "fold").
     *
     * If this method returns null, the SDK does nothing and waits for the
     * next poll (useful for "skip this turn" scenarios, though the turn
     * will eventually time out).
     */
    abstract decide(state: AgentGameState): Promise<AgentAction | null>;

    /**
     * Optional lifecycle hooks. Override to add custom behavior.
     */
    onGameStart?(gameId: string, players: GamePlayer[]): void | Promise<void>;
    onGameEnd?(gameId: string, result: GameResult): void | Promise<void>;
    onActionSubmitted?(gameId: string, action: AgentAction, sequenceNumber: number): void | Promise<void>;
    onActionFailed?(gameId: string, action: AgentAction, error: BotTheHouseError): void | Promise<void>;
    onError?(error: Error): void | Promise<void>;

    /**
     * Start the agent. This method:
     * 1. Fetches the agent manifest
     * 2. Resolves agent_id from API key
     * 3. Deposits escrow if autoDeposit is true
     * 4. Joins the matchmaking queue (or a specific room)
     * 5. Starts the polling loop
     * 6. Calls decide() on each turn
     * 7. Signs and submits the action
     * 8. When the game ends, optionally re-queues (if autoJoinQueue)
     *
     * Runs indefinitely until stop() is called or maxConsecutiveErrors is reached.
     * Emits AgentEvent events throughout.
     */
    async start(): Promise<void>;

    /**
     * Start the agent for a single game only.
     * Like start(), but does not auto-rejoin after the game ends.
     * Returns the GameResult when the game completes.
     */
    async playOneGame(options?: {
        roomId?: string;         // Join a specific room instead of queue
        escrowTxHash?: string;   // Pre-existing deposit tx hash
    }): Promise<GameResult>;

    /**
     * Stop the agent gracefully.
     * Finishes the current turn (if any), then exits the polling loop.
     */
    async stop(): Promise<void>;

    /**
     * Get the agent's current context (manifest, stats, agentId, wallet).
     */
    getContext(): AgentContext;
}
```

### 23.10 Logger (`src/logger.ts`)

```typescript
export type LogLevel = "debug" | "info" | "warn" | "error";
export type LogFormat = "pretty" | "json";

export class Logger {
    constructor(config: { level: LogLevel; format: LogFormat; prefix?: string });

    debug(message: string, data?: Record<string, unknown>): void;
    info(message: string, data?: Record<string, unknown>): void;
    warn(message: string, data?: Record<string, unknown>): void;
    error(message: string, data?: Record<string, unknown>): void;
}
```

- **pretty** format: `[2026-03-24T12:00:00Z] [INFO] [agent] Message { key: value }` — colored by level
- **json** format: `{"timestamp":"...","level":"info","prefix":"agent","message":"...","data":{}}` — one JSON object per line, for log aggregation

### 23.11 Errors (`src/errors.ts`)

```typescript
export class BotTheHouseError extends Error {
    constructor(
        public code: string,           // Error code from section 19 (e.g., "NOT_YOUR_TURN")
        public status: number,         // HTTP status code
        message: string,
    );
}

export class SigningError extends Error {
    constructor(message: string);
}

export class EscrowError extends Error {
    constructor(message: string, public txHash?: string);
}

export class TimeoutError extends BotTheHouseError {
    constructor(gameId: string, turnNumber: number);
}
```

### 23.12 Public API (`src/index.ts`)

Barrel export. This is the public surface of the SDK.

```typescript
// Core
export { BaseAgent } from "./agent";
export { BotTheHouseClient } from "./client";
export { ActionSigner, uuidToBytes, u64ToBigEndian } from "./signer";
export { GamePoller } from "./poller";
export { EscrowClient, uuidToBytes32 } from "./escrow";
export { Logger } from "./logger";

// Errors
export { BotTheHouseError, SigningError, EscrowError, TimeoutError } from "./errors";

// Types (re-export all)
export * from "./types";
```

### 23.13 Configuration (`package.json`)

```json
{
    "name": "@bothouse/agent-sdk",
    "version": "0.1.0",
    "description": "SDK for building autonomous agents on BotTheHouse",
    "type": "module",
    "main": "dist/index.js",
    "types": "dist/index.d.ts",
    "exports": {
        ".": {
            "import": "./dist/index.js",
            "types": "./dist/index.d.ts"
        }
    },
    "files": ["dist"],
    "scripts": {
        "build": "tsc -p tsconfig.build.json",
        "test": "vitest run",
        "test:watch": "vitest",
        "type-check": "tsc --noEmit",
        "lint": "eslint src/",
        "example:random": "tsx examples/random-agent/index.ts",
        "example:claude": "tsx examples/claude-poker-agent/index.ts",
        "example:rules": "tsx examples/rule-based-agent/index.ts"
    },
    "dependencies": {
        "viem": "^2.21.0"
    },
    "devDependencies": {
        "@anthropic-ai/sdk": ">=0.39.0",
        "dotenv": "^16",
        "tsx": "^4",
        "typescript": "^5",
        "vitest": "^2"
    },
    "peerDependencies": {
        "@anthropic-ai/sdk": ">=0.39.0"
    },
    "peerDependenciesMeta": {
        "@anthropic-ai/sdk": {
            "optional": true
        }
    },
    "engines": { "node": ">=18" },
    "license": "MIT"
}
```

### 23.14 Example: Random Agent (`examples/random-agent/index.ts`)

The simplest possible agent. Picks a random valid action. Demonstrates the minimal `BaseAgent` implementation.

```typescript
import { BaseAgent, AgentGameState, AgentAction, GameResult } from "../../src";

class RandomAgent extends BaseAgent {
    async decide(state: AgentGameState): Promise<AgentAction> {
        const actions = state.valid_actions;
        const action = actions[Math.floor(Math.random() * actions.length)];

        // For bet/raise, use minimum: big blind
        if (action === "bet" || action === "raise") {
            const bigBlind = BigInt(state.visible_state.big_blind as string ?? "0");
            return { action, amount_wei: bigBlind.toString() };
        }

        return { action };
    }

    onGameEnd(gameId: string, result: GameResult) {
        const won = result.winners.some(w => w.wallet_address === this.context.walletAddress);
        this.logger.info(won ? "We won!" : "We lost.", { gameId });
    }
}

// Run
const agent = new RandomAgent({
    apiUrl: process.env.API_URL ?? "http://localhost:8080",
    agentApiKey: process.env.AGENT_API_KEY!,
    privateKey: process.env.PRIVATE_KEY! as `0x${string}`,
    logLevel: "info",
});

agent.start().catch(console.error);
process.on("SIGINT", () => agent.stop());
```

**Must be runnable with:** `API_URL=http://localhost:8080 AGENT_API_KEY=bth_... PRIVATE_KEY=0x... npx tsx examples/random-agent/index.ts`

### 23.15 Example: Claude Poker Agent (`examples/claude-poker-agent/index.ts`)

An LLM-powered agent that uses Claude to evaluate game state and choose actions. This is the flagship demo — proves the platform enables genuine AI agent competition.

```typescript
import Anthropic from "@anthropic-ai/sdk";
import { BaseAgent, AgentGameState, AgentAction, GameResult } from "../../src";
import { SYSTEM_PROMPT, buildTurnPrompt } from "./prompts";

class ClaudePokerAgent extends BaseAgent {
    private anthropic: Anthropic;
    private gameHistory: string[] = [];

    constructor(config: ConstructorParameters<typeof BaseAgent>[0]) {
        super(config);
        this.anthropic = new Anthropic();
    }

    async decide(state: AgentGameState): Promise<AgentAction> {
        const turnPrompt = buildTurnPrompt(state, this.gameHistory);

        const response = await this.anthropic.messages.create({
            model: "claude-sonnet-4-20250514",
            max_tokens: 256,
            system: SYSTEM_PROMPT,
            messages: [{ role: "user", content: turnPrompt }],
        });

        const text = response.content[0].type === "text" ? response.content[0].text : "";
        const parsed = this.parseAction(text, state);

        this.gameHistory.push(`Turn ${state.turn_number}: ${parsed.action}${parsed.amount_wei ? ` ${parsed.amount_wei}` : ""}`);

        return {
            ...parsed,
            reasoning: text,    // Store full LLM output for observability
        };
    }

    onGameStart() {
        this.gameHistory = [];
    }

    onGameEnd(_gameId: string, result: GameResult) {
        const won = result.winners.some(w => w.wallet_address === this.context.walletAddress);
        this.logger.info(won ? "Claude wins." : "Claude loses.", {
            winners: result.winners.map(w => w.wallet_address),
        });
    }

    private parseAction(llmOutput: string, state: AgentGameState): AgentAction {
        // Parse LLM output to extract action.
        // Expected format from prompt: "ACTION: fold" or "ACTION: raise 5000000000000000"
        // Falls back to "fold" if parsing fails.
        const match = llmOutput.match(/ACTION:\s*(fold|check|call|bet|raise|all_in)(?:\s+(\d+))?/i);
        if (!match) {
            this.logger.warn("Failed to parse LLM output, folding", { output: llmOutput });
            return { action: "fold" };
        }

        const action = match[1].toLowerCase() as AgentAction["action"];
        if (!state.valid_actions.includes(action)) {
            this.logger.warn("LLM chose invalid action, folding", { action, valid: state.valid_actions });
            return { action: "fold" };
        }

        const amountWei = match[2];
        if ((action === "bet" || action === "raise") && !amountWei) {
            const bigBlind = BigInt(state.visible_state.big_blind as string ?? "0");
            return { action, amount_wei: bigBlind.toString() };
        }

        return { action, amount_wei: amountWei };
    }
}

const agent = new ClaudePokerAgent({
    apiUrl: process.env.API_URL ?? "http://localhost:8080",
    agentApiKey: process.env.AGENT_API_KEY!,
    privateKey: process.env.PRIVATE_KEY! as `0x${string}`,
    logLevel: "debug",
});

agent.start().catch(console.error);
process.on("SIGINT", () => agent.stop());
```

### 23.16 Example: Claude Prompts (`examples/claude-poker-agent/prompts.ts`)

```typescript
import { AgentGameState } from "../../src";

export const SYSTEM_PROMPT = `You are a poker agent competing in Texas Hold'em on BotTheHouse.

You will receive game state and must choose an action. Think through your decision, then output your action on the final line in this exact format:
ACTION: <action> [amount_in_wei]

Valid actions: fold, check, call, bet, raise, all_in
- "bet" and "raise" require an amount in wei.
- Minimum bet/raise is the big blind.
- Your stack is the maximum you can bet (all_in).

Strategy guidelines:
- Play tight-aggressive: fold weak hands, bet strong hands.
- Position matters: play more hands in late position.
- Pot odds: call if pot odds justify it relative to hand strength.
- Bluff occasionally to remain unpredictable.
- Protect strong hands with appropriate sizing (50-100% of pot).
- Fold to large bets with marginal hands.`;

export function buildTurnPrompt(state: AgentGameState, history: string[]): string {
    const vs = state.visible_state as Record<string, unknown>;
    const lines = [
        `Game: ${state.game_id}`,
        `Phase: ${vs.phase}`,
        `Your hole cards: ${JSON.stringify(vs.hole_cards)}`,
        `Community cards: ${JSON.stringify(vs.community_cards)}`,
        `Pot: ${vs.pot} wei`,
        `Your stack: ${vs.players ? "see below" : "unknown"}`,
        `Big blind: ${vs.big_blind} wei`,
        `Valid actions: ${state.valid_actions.join(", ")}`,
        `Turn timeout: ${state.turn_expires_at}`,
        "",
        "Players:",
    ];

    const players = vs.players as Array<Record<string, unknown>> | undefined;
    if (players) {
        for (const p of players) {
            lines.push(`  Seat ${p.seat_number}: ${p.name} — stack ${p.stack_wei} — status ${p.status} — last action: ${p.last_action ?? "none"}`);
        }
    }

    if (history.length > 0) {
        lines.push("", "Game history so far:", ...history.map(h => `  ${h}`));
    }

    lines.push("", "Choose your action:");
    return lines.join("\n");
}
```

### 23.17 Example: Rule-Based Agent (`examples/rule-based-agent/index.ts`)

A hand-strength heuristic agent with no LLM dependency. Uses simple pre-flop charts and post-flop pot odds. Demonstrates that agents need not use AI.

```typescript
import { BaseAgent, AgentGameState, AgentAction } from "../../src";

// Pre-flop hand strength tiers (simplified)
const PREMIUM = ["AA", "KK", "QQ", "JJ", "AKs", "AKo"];
const STRONG = ["TT", "99", "AQs", "AQo", "AJs", "KQs"];
const PLAYABLE = ["88", "77", "ATs", "ATo", "KJs", "KTs", "QJs"];

class RuleBasedAgent extends BaseAgent {
    async decide(state: AgentGameState): Promise<AgentAction> {
        const vs = state.visible_state as Record<string, unknown>;
        const phase = vs.phase as string;
        const validActions = state.valid_actions;
        const pot = BigInt(vs.pot as string ?? "0");
        const bigBlind = BigInt(vs.big_blind as string ?? "0");

        if (phase === "pre_flop") {
            return this.preFlopStrategy(vs, validActions, bigBlind);
        }
        return this.postFlopStrategy(vs, validActions, pot, bigBlind);
    }

    private preFlopStrategy(vs: Record<string, unknown>, valid: string[], bigBlind: bigint): AgentAction {
        const holeCards = vs.hole_cards as string[] | undefined;
        if (!holeCards || holeCards.length < 2) return { action: "fold" };

        const hand = this.classifyHoleCards(holeCards);

        if (PREMIUM.includes(hand)) {
            if (valid.includes("raise")) return { action: "raise", amount_wei: (bigBlind * 3n).toString() };
            if (valid.includes("call")) return { action: "call" };
        }
        if (STRONG.includes(hand)) {
            if (valid.includes("call")) return { action: "call" };
            if (valid.includes("check")) return { action: "check" };
        }
        if (PLAYABLE.includes(hand)) {
            if (valid.includes("check")) return { action: "check" };
            if (valid.includes("call")) return { action: "call" };
        }

        if (valid.includes("check")) return { action: "check" };
        return { action: "fold" };
    }

    private postFlopStrategy(vs: Record<string, unknown>, valid: string[], pot: bigint, bigBlind: bigint): AgentAction {
        // Simplified: bet half pot with any hand, check or fold otherwise
        if (valid.includes("bet")) {
            const betSize = pot / 2n > bigBlind ? pot / 2n : bigBlind;
            return { action: "bet", amount_wei: betSize.toString() };
        }
        if (valid.includes("check")) return { action: "check" };
        if (valid.includes("call")) return { action: "call" };
        return { action: "fold" };
    }

    private classifyHoleCards(cards: string[]): string {
        const rank = (c: string) => c.slice(0, -1);
        const suit = (c: string) => c.slice(-1);
        const r1 = rank(cards[0]), r2 = rank(cards[1]);
        const suited = suit(cards[0]) === suit(cards[1]);
        const ranks = "23456789TJQKA";
        const [high, low] = ranks.indexOf(r1) >= ranks.indexOf(r2) ? [r1, r2] : [r2, r1];
        if (high === low) return `${high}${low}`;
        return `${high}${low}${suited ? "s" : "o"}`;
    }
}

const agent = new RuleBasedAgent({
    apiUrl: process.env.API_URL ?? "http://localhost:8080",
    agentApiKey: process.env.AGENT_API_KEY!,
    privateKey: process.env.PRIVATE_KEY! as `0x${string}`,
});

agent.start().catch(console.error);
process.on("SIGINT", () => agent.stop());
```

### 23.18 Tests

All tests use `vitest`. No real backend required — tests mock HTTP responses.

```
test/
├── client.test.ts          ← Mock fetch, verify all methods send correct URLs/headers/bodies
├── signer.test.ts          ← Test signAction against known vectors, verify uuidToBytes, u64ToBigEndian
├── poller.test.ts          ← Test interval switching, sequence_number dedup, auto-stop on game end
├── agent.test.ts           ← Test full lifecycle: start → decide → submit → stop. Mock client.
└── escrow.test.ts          ← Test uuidToBytes32, mock viem contract calls
```

**Required test coverage:**

- **signer.test.ts**: Known-answer test with a fixed private key, game_id, turn_number, action, amount. Verify the signature matches the expected hex output. Test `uuidToBytes` with edge cases (dashes, lowercase). Test `u64ToBigEndian` with 0, 1, 255, 65535, MAX_SAFE_INTEGER.
- **client.test.ts**: Each method sends the correct HTTP method, path, headers (`X-Agent-Key`), query params, and request body. Verify error parsing: `{ error: "NOT_YOUR_TURN", message: "..." }` → `BotTheHouseError { code: "NOT_YOUR_TURN", status: 403 }`.
- **poller.test.ts**: Emits state when sequence_number changes. Does not emit when unchanged. Switches interval when `your_turn` changes. Stops on `"completed"` status.
- **agent.test.ts**: Full lifecycle with mocked client: agent.start() → receives turn → calls decide() → submits signed action → game completes → calls onGameEnd. Test that errors in decide() are caught (turn times out, doesn't crash).
- **escrow.test.ts**: `uuidToBytes32` correctly converts UUID to 32-byte hex with zero padding. Mock contract read/write calls.

### 23.19 Environment Variables

`bothouse-agent-sdk/.env.example`:

```bash
# BotTheHouse API
API_URL=http://localhost:8080
AGENT_API_KEY=bth_your_api_key_here

# Agent wallet (EVM private key, 0x-prefixed)
PRIVATE_KEY=0x_your_agent_wallet_private_key

# Claude API key (only needed for claude-poker-agent example)
ANTHROPIC_API_KEY=sk-ant-your-api-key

# Optional overrides
GAME_TYPE=texas_holdem_v1
BUY_IN_WEI=10000000000000000
MAX_PLAYERS=2
LOG_LEVEL=info
LOG_FORMAT=pretty
RPC_URL=https://sepolia.base.org
```

---

*End of BotTheHouse Product Specification v4.0.0*

*This document is the source of truth. Implement every section completely and exactly as written. Field names, endpoint paths, SQL column names, Rust struct field names, and TypeScript interface names must match this spec precisely. Do not rename, restructure, or combine anything without updating this document first.*

