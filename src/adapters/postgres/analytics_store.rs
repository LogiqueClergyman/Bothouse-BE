use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::analytics::{
    AgentActionEntry, AgentHandSummary, AgentMetrics, H2HMatchRecord, H2HTendencies,
    HeadToHeadRecord,
};
use crate::errors::AppError;
use crate::ports::analytics_store::AnalyticsStore;

pub struct PgAnalyticsStore {
    pool: PgPool,
}

impl PgAnalyticsStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ─── Row types ────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct AgentMetricsRow {
    agent_id: Uuid,
    game_type: String,
    hands_dealt: i64,
    vpip_count: i64,
    pfr_count: i64,
    three_bet_count: i64,
    fold_to_3bet_count: i64,
    fold_to_3bet_opps: i64,
    cbet_count: i64,
    cbet_opps: i64,
    fold_to_cbet_count: i64,
    fold_to_cbet_opps: i64,
    steal_count: i64,
    steal_opps: i64,
    aggressive_actions: i64,
    passive_actions: i64,
    went_to_showdown: i64,
    won_at_showdown: i64,
    total_bb_won: i64,
    total_bb_hands: i64,
    updated_at: DateTime<Utc>,
}

impl From<AgentMetricsRow> for AgentMetrics {
    fn from(r: AgentMetricsRow) -> Self {
        AgentMetrics {
            agent_id: r.agent_id,
            game_type: r.game_type,
            hands_dealt: r.hands_dealt,
            vpip_count: r.vpip_count,
            pfr_count: r.pfr_count,
            three_bet_count: r.three_bet_count,
            fold_to_3bet_count: r.fold_to_3bet_count,
            fold_to_3bet_opps: r.fold_to_3bet_opps,
            cbet_count: r.cbet_count,
            cbet_opps: r.cbet_opps,
            fold_to_cbet_count: r.fold_to_cbet_count,
            fold_to_cbet_opps: r.fold_to_cbet_opps,
            steal_count: r.steal_count,
            steal_opps: r.steal_opps,
            aggressive_actions: r.aggressive_actions,
            passive_actions: r.passive_actions,
            went_to_showdown: r.went_to_showdown,
            won_at_showdown: r.won_at_showdown,
            total_bb_won: r.total_bb_won,
            total_bb_hands: r.total_bb_hands,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ActionRow {
    game_id: Uuid,
    hand_number: Option<i32>,
    phase: Option<String>,
    sequence: i64,
    timestamp: DateTime<Utc>,
    action: String,
    amount_wei: Option<String>,
    pot_before_action: Option<String>,
    stack_before_action: Option<String>,
    num_players_in_hand: Option<i16>,
    dealer_seat: Option<i16>,
    seat_number: Option<i16>,
}

impl ActionRow {
    fn into_entry(self) -> AgentActionEntry {
        let position = compute_position(self.dealer_seat, self.seat_number, None);
        AgentActionEntry {
            game_id: self.game_id,
            hand_number: self.hand_number,
            phase: self.phase,
            turn_number: self.sequence,
            action: self.action,
            amount_wei: self.amount_wei,
            pot_before_action_wei: self.pot_before_action,
            stack_before_action_wei: self.stack_before_action,
            num_players_in_hand: self.num_players_in_hand,
            position,
            timestamp: self.timestamp,
        }
    }
}

#[derive(sqlx::FromRow)]
struct H2HRow {
    opponent_id: Uuid,
    opponent_name: String,
    game_type: String,
    games_together: i32,
    hands_together: i32,
    agent_hands_won: i32,
    opponent_hands_won: i32,
    split_hands: i32,
    net_profit_wei: Option<String>,
    agent_vpip_count: i32,
    agent_vpip_opps: i32,
    agent_pfr_count: i32,
    agent_agg_bets: i32,
    agent_agg_calls: i32,
    agent_fold_to_raise_count: i32,
    agent_fold_to_raise_opps: i32,
    updated_at: DateTime<Utc>,
}

/// Compute position name from dealer_seat and seat_number.
fn compute_position(
    dealer_seat: Option<i16>,
    seat_number: Option<i16>,
    _num_players: Option<i16>,
) -> Option<String> {
    // Position computation requires both values; simplified for now
    let _ds = dealer_seat?;
    let _sn = seat_number?;
    // Full position computation would use num_players and table geometry.
    // For now, return None — will be enriched by the computation worker.
    None
}

fn compute_h2h_tendencies(
    vpip_count: i32,
    vpip_opps: i32,
    pfr_count: i32,
    agg_bets: i32,
    agg_calls: i32,
    fold_to_raise_count: i32,
    fold_to_raise_opps: i32,
) -> H2HTendencies {
    let pct = |n: i32, d: i32| -> f64 {
        if d == 0 { 0.0 } else { n as f64 / d as f64 * 100.0 }
    };
    H2HTendencies {
        vpip: pct(vpip_count, vpip_opps),
        pfr: pct(pfr_count, vpip_opps),
        aggression_factor: if agg_calls == 0 {
            agg_bets as f64
        } else {
            agg_bets as f64 / agg_calls as f64
        },
        fold_to_raise: pct(fold_to_raise_count, fold_to_raise_opps),
    }
}

// ─── Impl ─────────────────────────────────────────────────────────────────────

#[async_trait]
impl AnalyticsStore for PgAnalyticsStore {
    async fn get_metrics(
        &self,
        agent_id: Uuid,
        game_type: &str,
    ) -> Result<Option<AgentMetrics>, AppError> {
        let row = sqlx::query_as::<_, AgentMetricsRow>(
            r#"
            SELECT agent_id, game_type, hands_dealt, vpip_count, pfr_count,
                   three_bet_count, fold_to_3bet_count, fold_to_3bet_opps,
                   cbet_count, cbet_opps, fold_to_cbet_count, fold_to_cbet_opps,
                   steal_count, steal_opps, aggressive_actions, passive_actions,
                   went_to_showdown, won_at_showdown, total_bb_won, total_bb_hands,
                   updated_at
            FROM agent_metrics
            WHERE agent_id = $1 AND game_type = $2
            "#,
        )
        .bind(agent_id)
        .bind(game_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(row.map(AgentMetrics::from))
    }

    async fn upsert_metrics(&self, m: &AgentMetrics) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO agent_metrics (
                agent_id, game_type, hands_dealt, vpip_count, pfr_count,
                three_bet_count, fold_to_3bet_count, fold_to_3bet_opps,
                cbet_count, cbet_opps, fold_to_cbet_count, fold_to_cbet_opps,
                steal_count, steal_opps, aggressive_actions, passive_actions,
                went_to_showdown, won_at_showdown, total_bb_won, total_bb_hands,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, $21
            )
            ON CONFLICT (agent_id, game_type) DO UPDATE SET
                hands_dealt           = EXCLUDED.hands_dealt,
                vpip_count            = EXCLUDED.vpip_count,
                pfr_count             = EXCLUDED.pfr_count,
                three_bet_count       = EXCLUDED.three_bet_count,
                fold_to_3bet_count    = EXCLUDED.fold_to_3bet_count,
                fold_to_3bet_opps     = EXCLUDED.fold_to_3bet_opps,
                cbet_count            = EXCLUDED.cbet_count,
                cbet_opps             = EXCLUDED.cbet_opps,
                fold_to_cbet_count    = EXCLUDED.fold_to_cbet_count,
                fold_to_cbet_opps     = EXCLUDED.fold_to_cbet_opps,
                steal_count           = EXCLUDED.steal_count,
                steal_opps            = EXCLUDED.steal_opps,
                aggressive_actions    = EXCLUDED.aggressive_actions,
                passive_actions       = EXCLUDED.passive_actions,
                went_to_showdown      = EXCLUDED.went_to_showdown,
                won_at_showdown       = EXCLUDED.won_at_showdown,
                total_bb_won          = EXCLUDED.total_bb_won,
                total_bb_hands        = EXCLUDED.total_bb_hands,
                updated_at            = EXCLUDED.updated_at
            "#,
        )
        .bind(m.agent_id)
        .bind(&m.game_type)
        .bind(m.hands_dealt)
        .bind(m.vpip_count)
        .bind(m.pfr_count)
        .bind(m.three_bet_count)
        .bind(m.fold_to_3bet_count)
        .bind(m.fold_to_3bet_opps)
        .bind(m.cbet_count)
        .bind(m.cbet_opps)
        .bind(m.fold_to_cbet_count)
        .bind(m.fold_to_cbet_opps)
        .bind(m.steal_count)
        .bind(m.steal_opps)
        .bind(m.aggressive_actions)
        .bind(m.passive_actions)
        .bind(m.went_to_showdown)
        .bind(m.won_at_showdown)
        .bind(m.total_bb_won)
        .bind(m.total_bb_hands)
        .bind(m.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(())
    }

    async fn list_actions(
        &self,
        agent_id: Uuid,
        _game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentActionEntry>, AppError> {
        let rows = sqlx::query_as::<_, ActionRow>(
            r#"
            SELECT gl.game_id, gl.hand_number,
                   gl.phase, gl.sequence,
                   gl.timestamp, gl.action, gl.amount_wei::TEXT as amount_wei,
                   gl.pot_before_action::TEXT as pot_before_action,
                   gl.stack_before_action::TEXT as stack_before_action,
                   gl.num_players_in_hand, gl.dealer_seat,
                   gp.seat_number
            FROM game_log gl
            LEFT JOIN game_players gp ON gp.game_id = gl.game_id AND gp.agent_id = gl.agent_id
            WHERE gl.agent_id = $1
            ORDER BY gl.timestamp DESC, gl.sequence DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(agent_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(rows.into_iter().map(ActionRow::into_entry).collect())
    }

    async fn list_hands(
        &self,
        agent_id: Uuid,
        _game_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentHandSummary>, AppError> {
        #[derive(sqlx::FromRow)]
        struct HandRow {
            game_id: Uuid,
            hand_number: i32,
            went_to_showdown: Option<bool>,
            actions_json: Option<serde_json::Value>,
            max_timestamp: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, HandRow>(
            r#"
            SELECT
                gl.game_id,
                gl.hand_number,
                BOOL_OR(gl.action = 'showdown') AS went_to_showdown,
                json_agg(gl.action ORDER BY gl.sequence) AS actions_json,
                MAX(gl.timestamp) AS max_timestamp
            FROM game_log gl
            WHERE gl.agent_id = $1 AND gl.hand_number IS NOT NULL
            GROUP BY gl.game_id, gl.hand_number
            ORDER BY MAX(gl.timestamp) DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(agent_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        let summaries = rows
            .into_iter()
            .map(|r| {
                let went_to_showdown = r.went_to_showdown.unwrap_or(false);
                let actions_taken: Vec<String> = r
                    .actions_json
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let vpip = actions_taken
                    .iter()
                    .any(|a| a == "call" || a == "raise" || a == "bet");
                let pfr = actions_taken.iter().any(|a| a == "raise");
                AgentHandSummary {
                    game_id: r.game_id,
                    hand_number: r.hand_number,
                    position: None,
                    hole_cards: if went_to_showdown {
                        None // Populated by enrichment step
                    } else {
                        None
                    },
                    final_phase: None,
                    went_to_showdown,
                    result: "unknown".to_string(),
                    profit_wei: None,
                    pot_wei: None,
                    actions_taken,
                    vpip,
                    pfr,
                    timestamp: r.max_timestamp,
                }
            })
            .collect();

        Ok(summaries)
    }

    async fn get_head_to_head(
        &self,
        agent_id: Uuid,
        opponent_id: Uuid,
        game_type: &str,
    ) -> Result<Option<HeadToHeadRecord>, AppError> {
        // Query the agent's row
        let agent_row = sqlx::query_as::<_, H2HRow>(
            r#"
            SELECT h.opponent_id, a.name as opponent_name, h.game_type,
                   h.games_together, h.hands_together,
                   h.agent_hands_won, h.opponent_hands_won, h.split_hands,
                   h.agent_net_profit_wei::TEXT as net_profit_wei,
                   h.agent_vpip_count, h.agent_vpip_opps,
                   h.agent_pfr_count, h.agent_agg_bets, h.agent_agg_calls,
                   h.agent_fold_to_raise_count, h.agent_fold_to_raise_opps,
                   h.updated_at
            FROM agent_head_to_head h
            JOIN agents a ON a.agent_id = h.opponent_id
            WHERE h.agent_id = $1 AND h.opponent_id = $2 AND h.game_type = $3
            "#,
        )
        .bind(agent_id)
        .bind(opponent_id)
        .bind(game_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        let agent_row = match agent_row {
            Some(r) => r,
            None => return Ok(None),
        };

        // Query the reverse row for opponent's tendencies vs this agent
        let opp_row = sqlx::query_as::<_, H2HRow>(
            r#"
            SELECT h.opponent_id, a.name as opponent_name, h.game_type,
                   h.games_together, h.hands_together,
                   h.agent_hands_won, h.opponent_hands_won, h.split_hands,
                   h.agent_net_profit_wei::TEXT as net_profit_wei,
                   h.agent_vpip_count, h.agent_vpip_opps,
                   h.agent_pfr_count, h.agent_agg_bets, h.agent_agg_calls,
                   h.agent_fold_to_raise_count, h.agent_fold_to_raise_opps,
                   h.updated_at
            FROM agent_head_to_head h
            JOIN agents a ON a.agent_id = h.opponent_id
            WHERE h.agent_id = $1 AND h.opponent_id = $2 AND h.game_type = $3
            "#,
        )
        .bind(opponent_id)
        .bind(agent_id)
        .bind(game_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        // Get agent name
        let agent_name_row = sqlx::query_as::<_, (String,)>("SELECT name FROM agents WHERE agent_id = $1")
            .bind(agent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        let agent_name = agent_name_row.map(|r| r.0).unwrap_or_default();

        let agent_tendencies = compute_h2h_tendencies(
            agent_row.agent_vpip_count,
            agent_row.agent_vpip_opps,
            agent_row.agent_pfr_count,
            agent_row.agent_agg_bets,
            agent_row.agent_agg_calls,
            agent_row.agent_fold_to_raise_count,
            agent_row.agent_fold_to_raise_opps,
        );

        let opponent_tendencies = opp_row
            .map(|r| {
                compute_h2h_tendencies(
                    r.agent_vpip_count,
                    r.agent_vpip_opps,
                    r.agent_pfr_count,
                    r.agent_agg_bets,
                    r.agent_agg_calls,
                    r.agent_fold_to_raise_count,
                    r.agent_fold_to_raise_opps,
                )
            })
            .unwrap_or(H2HTendencies {
                vpip: 0.0,
                pfr: 0.0,
                aggression_factor: 0.0,
                fold_to_raise: 0.0,
            });

        Ok(Some(HeadToHeadRecord {
            agent_id,
            agent_name,
            opponent_id: agent_row.opponent_id,
            opponent_name: agent_row.opponent_name,
            game_type: agent_row.game_type,
            games_together: agent_row.games_together,
            hands_together: agent_row.hands_together,
            record: H2HMatchRecord {
                agent_hands_won: agent_row.agent_hands_won,
                opponent_hands_won: agent_row.opponent_hands_won,
                split: agent_row.split_hands,
            },
            agent_net_profit_wei: agent_row
                .net_profit_wei
                .unwrap_or_else(|| "0".to_string()),
            agent_tendencies_vs_opponent: agent_tendencies,
            opponent_tendencies_vs_agent: opponent_tendencies,
            computed_at: agent_row.updated_at,
        }))
    }

    async fn upsert_head_to_head(&self, r: &HeadToHeadRecord) -> Result<(), AppError> {
        let net_profit: sqlx::types::BigDecimal = r
            .agent_net_profit_wei
            .parse()
            .unwrap_or(sqlx::types::BigDecimal::from(0));

        sqlx::query(
            r#"
            INSERT INTO agent_head_to_head (
                agent_id, opponent_id, game_type, games_together, hands_together,
                agent_hands_won, opponent_hands_won, split_hands, agent_net_profit_wei,
                agent_vpip_count, agent_vpip_opps, agent_pfr_count,
                agent_agg_bets, agent_agg_calls,
                agent_fold_to_raise_count, agent_fold_to_raise_opps,
                updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (agent_id, opponent_id, game_type) DO UPDATE SET
                games_together               = EXCLUDED.games_together,
                hands_together               = EXCLUDED.hands_together,
                agent_hands_won              = EXCLUDED.agent_hands_won,
                opponent_hands_won           = EXCLUDED.opponent_hands_won,
                split_hands                  = EXCLUDED.split_hands,
                agent_net_profit_wei         = EXCLUDED.agent_net_profit_wei,
                agent_vpip_count             = EXCLUDED.agent_vpip_count,
                agent_vpip_opps              = EXCLUDED.agent_vpip_opps,
                agent_pfr_count              = EXCLUDED.agent_pfr_count,
                agent_agg_bets               = EXCLUDED.agent_agg_bets,
                agent_agg_calls              = EXCLUDED.agent_agg_calls,
                agent_fold_to_raise_count    = EXCLUDED.agent_fold_to_raise_count,
                agent_fold_to_raise_opps     = EXCLUDED.agent_fold_to_raise_opps,
                updated_at                   = EXCLUDED.updated_at
            "#,
        )
        .bind(r.agent_id)
        .bind(r.opponent_id)
        .bind(&r.game_type)
        .bind(r.games_together)
        .bind(r.hands_together)
        .bind(r.record.agent_hands_won)
        .bind(r.record.opponent_hands_won)
        .bind(r.record.split)
        .bind(net_profit)
        .bind(0_i32) // agent_vpip_count — from raw tendencies
        .bind(0_i32) // agent_vpip_opps
        .bind(0_i32) // agent_pfr_count
        .bind(0_i32) // agent_agg_bets
        .bind(0_i32) // agent_agg_calls
        .bind(0_i32) // agent_fold_to_raise_count
        .bind(0_i32) // agent_fold_to_raise_opps
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(())
    }
}
