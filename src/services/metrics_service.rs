use uuid::Uuid;

use crate::domain::analytics::{
    AdvancedMetrics, AgentActionEntry, AgentHandSummary, AgentMetrics, AgentTendencies,
    CoreMetrics, H2HMatchRecord, H2HTendencies, HeadToHeadRecord, PlayStyle, PlayStyleSummary,
};
use crate::errors::AppError;
use crate::state::AppState;

const DEFAULT_GAME_TYPE: &str = "texas_holdem_v1";

/// Compute an `AgentTendencies` from raw `AgentMetrics` counters.
pub fn compute_tendencies(m: &AgentMetrics, agent_name: String) -> AgentTendencies {
    let pct = |num: i64, den: i64| -> f64 {
        if den == 0 {
            0.0
        } else {
            num as f64 / den as f64 * 100.0
        }
    };

    let vpip = pct(m.vpip_count, m.hands_dealt);
    let pfr = pct(m.pfr_count, m.hands_dealt);
    let aggression_factor = if m.passive_actions == 0 {
        m.aggressive_actions as f64
    } else {
        m.aggressive_actions as f64 / m.passive_actions as f64
    };
    let wtsd = pct(m.went_to_showdown, m.hands_dealt);
    let wsd = pct(m.won_at_showdown, m.went_to_showdown);
    let three_bet_pct = pct(m.three_bet_count, m.hands_dealt);
    let fold_to_three_bet = pct(m.fold_to_3bet_count, m.fold_to_3bet_opps);
    let cbet_pct = pct(m.cbet_count, m.cbet_opps);
    let fold_to_cbet = pct(m.fold_to_cbet_count, m.fold_to_cbet_opps);
    let steal_pct = pct(m.steal_count, m.steal_opps);
    let bb_per_100 = if m.total_bb_hands == 0 {
        0.0
    } else {
        m.total_bb_won as f64 / m.total_bb_hands as f64
    };

    let play_style = PlayStyle::classify(vpip, aggression_factor);

    AgentTendencies {
        agent_id: m.agent_id,
        agent_name,
        game_type: m.game_type.clone(),
        sample_size: m.hands_dealt,
        computed_at: m.updated_at,
        core: CoreMetrics {
            vpip,
            pfr,
            aggression_factor,
            wtsd,
            w_usd_sd: wsd,
        },
        advanced: AdvancedMetrics {
            three_bet_pct,
            fold_to_three_bet,
            cbet_pct,
            fold_to_cbet,
            steal_pct,
            bb_per_100,
        },
        summary: PlayStyleSummary {
            play_style: play_style.abbreviation().to_string(),
            play_style_label: play_style.label().to_string(),
            description: play_style.description().to_string(),
        },
        last_updated_hand: m.hands_dealt,
    }
}

pub async fn get_tendencies(
    agent_id: Uuid,
    game_type: Option<String>,
    state: &AppState,
) -> Result<AgentTendencies, AppError> {
    let agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let gt = game_type.as_deref().unwrap_or(DEFAULT_GAME_TYPE);
    let metrics = state
        .analytics_store
        .get_metrics(agent_id, gt)
        .await?
        .unwrap_or_else(|| AgentMetrics::empty(agent_id, gt.to_string()));

    Ok(compute_tendencies(&metrics, agent.name))
}

pub async fn list_actions(
    agent_id: Uuid,
    game_type: Option<String>,
    limit: i64,
    offset: i64,
    state: &AppState,
) -> Result<Vec<AgentActionEntry>, AppError> {
    state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let limit = limit.min(200);

    state
        .analytics_store
        .list_actions(agent_id, game_type.as_deref(), limit, offset)
        .await
}

pub async fn list_hands(
    agent_id: Uuid,
    game_type: Option<String>,
    limit: i64,
    offset: i64,
    state: &AppState,
) -> Result<Vec<AgentHandSummary>, AppError> {
    state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let limit = limit.min(200);

    state
        .analytics_store
        .list_hands(agent_id, game_type.as_deref(), limit, offset)
        .await
}

pub async fn get_head_to_head(
    agent_id: Uuid,
    opponent_id: Uuid,
    game_type: Option<String>,
    state: &AppState,
) -> Result<HeadToHeadRecord, AppError> {
    let agent = state
        .agent_store
        .get_agent_by_id(agent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let opponent = state
        .agent_store
        .get_agent_by_id(opponent_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let gt = game_type.as_deref().unwrap_or(DEFAULT_GAME_TYPE);

    if let Some(record) = state
        .analytics_store
        .get_head_to_head(agent_id, opponent_id, gt)
        .await?
    {
        return Ok(record);
    }

    // Return an empty placeholder — no games played yet
    Ok(HeadToHeadRecord {
        agent_id,
        agent_name: agent.name,
        opponent_id,
        opponent_name: opponent.name,
        game_type: gt.to_string(),
        games_together: 0,
        hands_together: 0,
        record: H2HMatchRecord {
            agent_hands_won: 0,
            opponent_hands_won: 0,
            split: 0,
        },
        agent_net_profit_wei: "0".to_string(),
        agent_tendencies_vs_opponent: H2HTendencies {
            vpip: 0.0,
            pfr: 0.0,
            aggression_factor: 0.0,
            fold_to_raise: 0.0,
        },
        opponent_tendencies_vs_agent: H2HTendencies {
            vpip: 0.0,
            pfr: 0.0,
            aggression_factor: 0.0,
            fold_to_raise: 0.0,
        },
        computed_at: chrono::Utc::now(),
    })
}
