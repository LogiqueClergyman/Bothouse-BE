use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Materialized per-agent poker metrics (one row per agent+game_type).
/// All percentages are stored as raw counters; the service computes rates at read time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: Uuid,
    pub game_type: String,

    // Raw counters
    pub hands_dealt: i64,
    pub vpip_count: i64,
    pub pfr_count: i64,
    pub three_bet_count: i64,
    pub fold_to_3bet_count: i64,
    pub fold_to_3bet_opps: i64,
    pub cbet_count: i64,
    pub cbet_opps: i64,
    pub fold_to_cbet_count: i64,
    pub fold_to_cbet_opps: i64,
    pub steal_count: i64,
    pub steal_opps: i64,
    pub aggressive_actions: i64,
    pub passive_actions: i64,
    pub went_to_showdown: i64,
    pub won_at_showdown: i64,
    pub total_bb_won: i64,
    pub total_bb_hands: i64,

    pub updated_at: DateTime<Utc>,
}

impl AgentMetrics {
    pub fn empty(agent_id: Uuid, game_type: String) -> Self {
        Self {
            agent_id,
            game_type,
            hands_dealt: 0,
            vpip_count: 0,
            pfr_count: 0,
            three_bet_count: 0,
            fold_to_3bet_count: 0,
            fold_to_3bet_opps: 0,
            cbet_count: 0,
            cbet_opps: 0,
            fold_to_cbet_count: 0,
            fold_to_cbet_opps: 0,
            steal_count: 0,
            steal_opps: 0,
            aggressive_actions: 0,
            passive_actions: 0,
            went_to_showdown: 0,
            won_at_showdown: 0,
            total_bb_won: 0,
            total_bb_hands: 0,
            updated_at: Utc::now(),
        }
    }
}

/// Computed tendencies exposed to API consumers.
/// Matches the spec shape: `core`, `advanced`, `summary` sections.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentTendencies {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub game_type: String,
    pub sample_size: i64,
    pub computed_at: DateTime<Utc>,
    pub core: CoreMetrics,
    pub advanced: AdvancedMetrics,
    pub summary: PlayStyleSummary,
    pub last_updated_hand: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoreMetrics {
    pub vpip: f64,
    pub pfr: f64,
    pub aggression_factor: f64,
    pub wtsd: f64,
    pub w_usd_sd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvancedMetrics {
    pub three_bet_pct: f64,
    pub fold_to_three_bet: f64,
    pub cbet_pct: f64,
    pub fold_to_cbet: f64,
    pub steal_pct: f64,
    pub bb_per_100: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayStyleSummary {
    pub play_style: String,
    pub play_style_label: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PlayStyle {
    Tag,
    Lag,
    Rock,
    CallingStation,
    Unknown,
}

impl PlayStyle {
    /// Spec threshold: VPIP 25%, AF 1.5
    pub fn classify(vpip: f64, aggression_factor: f64) -> Self {
        match (vpip >= 25.0, aggression_factor >= 1.5) {
            (false, true) => PlayStyle::Tag,
            (true, true) => PlayStyle::Lag,
            (false, false) => PlayStyle::Rock,
            (true, false) => PlayStyle::CallingStation,
        }
    }

    pub fn abbreviation(&self) -> &str {
        match self {
            PlayStyle::Tag => "TAG",
            PlayStyle::Lag => "LAG",
            PlayStyle::Rock => "Rock",
            PlayStyle::CallingStation => "Calling Station",
            PlayStyle::Unknown => "Unknown",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            PlayStyle::Tag => "Tight-Aggressive",
            PlayStyle::Lag => "Loose-Aggressive",
            PlayStyle::Rock => "Tight-Passive",
            PlayStyle::CallingStation => "Loose-Passive",
            PlayStyle::Unknown => "Unknown",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            PlayStyle::Tag => "Selective hand choice with aggressive post-flop play. Folds most hands but bets and raises when entering a pot.",
            PlayStyle::Lag => "Plays many hands aggressively. Hard to read, high variance. Applies constant pressure.",
            PlayStyle::Rock => "Plays few hands, rarely bets aggressively. Predictable. Easy to steal from.",
            PlayStyle::CallingStation => "Plays many hands, mostly calls. Easy to exploit with value bets. Never bluff.",
            PlayStyle::Unknown => "Insufficient data to classify play style.",
        }
    }
}

/// Per-matchup head-to-head record.
/// Spec shape: includes dual per-matchup tendencies.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadToHeadRecord {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub opponent_id: Uuid,
    pub opponent_name: String,
    pub game_type: String,
    pub games_together: i32,
    pub hands_together: i32,
    pub record: H2HMatchRecord,
    pub agent_net_profit_atomic: String,
    pub agent_tendencies_vs_opponent: H2HTendencies,
    pub opponent_tendencies_vs_agent: H2HTendencies,
    pub computed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct H2HMatchRecord {
    pub agent_hands_won: i32,
    pub opponent_hands_won: i32,
    pub split: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct H2HTendencies {
    pub vpip: f64,
    pub pfr: f64,
    pub aggression_factor: f64,
    pub fold_to_raise: f64,
}

/// A single action entry from the game log for a given agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentActionEntry {
    pub game_id: Uuid,
    pub hand_number: Option<i32>,
    pub phase: Option<String>,
    pub turn_number: i64,
    pub action: String,
    pub amount_atomic: Option<String>,
    pub pot_before_action_atomic: Option<String>,
    pub stack_before_action_atomic: Option<String>,
    pub num_players_in_hand: Option<i16>,
    pub position: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// High-level summary of a hand for a given agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentHandSummary {
    pub game_id: Uuid,
    pub hand_number: i32,
    pub position: Option<String>,
    pub hole_cards: Option<Vec<String>>,
    pub final_phase: Option<String>,
    pub went_to_showdown: bool,
    pub result: String, // "won" | "lost" | "unknown"
    pub profit_atomic: Option<String>,
    pub pot_atomic: Option<String>,
    pub actions_taken: Vec<String>,
    pub vpip: bool,
    pub pfr: bool,
    pub timestamp: DateTime<Utc>,
}
