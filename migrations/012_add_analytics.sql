-- Analytics: Add columns to game_log for position and phase tracking
ALTER TABLE game_log
    ADD COLUMN IF NOT EXISTS phase            TEXT,
    ADD COLUMN IF NOT EXISTS hand_number      INTEGER,
    ADD COLUMN IF NOT EXISTS pot_before_action TEXT,
    ADD COLUMN IF NOT EXISTS stack_before_action TEXT,
    ADD COLUMN IF NOT EXISTS num_players_in_hand SMALLINT,
    ADD COLUMN IF NOT EXISTS dealer_seat      SMALLINT,
    ADD COLUMN IF NOT EXISTS is_voluntary     BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_aggressive    BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_game_log_agent_action
    ON game_log(agent_id, action)
    WHERE agent_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_game_log_agent_hand
    ON game_log(agent_id, hand_number)
    WHERE agent_id IS NOT NULL;

-- Materialized metrics table: one row per (agent, game_type)
CREATE TABLE IF NOT EXISTS agent_metrics (
    agent_id          UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    game_type         VARCHAR(64) NOT NULL DEFAULT 'texas_holdem_v1',
    hands_dealt       BIGINT NOT NULL DEFAULT 0,
    vpip_count        BIGINT NOT NULL DEFAULT 0,
    pfr_count         BIGINT NOT NULL DEFAULT 0,
    three_bet_count   BIGINT NOT NULL DEFAULT 0,
    fold_to_3bet_count   BIGINT NOT NULL DEFAULT 0,
    fold_to_3bet_opps    BIGINT NOT NULL DEFAULT 0,
    cbet_count        BIGINT NOT NULL DEFAULT 0,
    cbet_opps         BIGINT NOT NULL DEFAULT 0,
    fold_to_cbet_count   BIGINT NOT NULL DEFAULT 0,
    fold_to_cbet_opps    BIGINT NOT NULL DEFAULT 0,
    steal_count       BIGINT NOT NULL DEFAULT 0,
    steal_opps        BIGINT NOT NULL DEFAULT 0,
    aggressive_actions BIGINT NOT NULL DEFAULT 0,
    passive_actions    BIGINT NOT NULL DEFAULT 0,
    went_to_showdown   BIGINT NOT NULL DEFAULT 0,
    won_at_showdown    BIGINT NOT NULL DEFAULT 0,
    total_bb_won       BIGINT NOT NULL DEFAULT 0,
    total_bb_hands     BIGINT NOT NULL DEFAULT 0,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (agent_id, game_type)
);

CREATE INDEX IF NOT EXISTS idx_agent_metrics_agent
    ON agent_metrics(agent_id);

-- Head-to-head records: one row per ordered pair (agent_id, opponent_id, game_type)
-- Query BOTH (A,B) and (B,A) rows to get full matchup (spec §24.9)
CREATE TABLE IF NOT EXISTS agent_head_to_head (
    agent_id            UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    opponent_id         UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    game_type           VARCHAR(64) NOT NULL DEFAULT 'texas_holdem_v1',
    games_together      INTEGER NOT NULL DEFAULT 0,
    hands_together      INTEGER NOT NULL DEFAULT 0,
    agent_hands_won     INTEGER NOT NULL DEFAULT 0,
    opponent_hands_won  INTEGER NOT NULL DEFAULT 0,
    split_hands         INTEGER NOT NULL DEFAULT 0,
    agent_net_profit_wei NUMERIC(78,0) NOT NULL DEFAULT 0,
    -- Matchup-specific raw counters for tendencies
    agent_vpip_count    INTEGER NOT NULL DEFAULT 0,
    agent_vpip_opps     INTEGER NOT NULL DEFAULT 0,
    agent_pfr_count     INTEGER NOT NULL DEFAULT 0,
    agent_agg_bets      INTEGER NOT NULL DEFAULT 0,
    agent_agg_calls     INTEGER NOT NULL DEFAULT 0,
    agent_fold_to_raise_count  INTEGER NOT NULL DEFAULT 0,
    agent_fold_to_raise_opps   INTEGER NOT NULL DEFAULT 0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (agent_id, opponent_id, game_type)
);

CREATE INDEX IF NOT EXISTS idx_h2h_agent ON agent_head_to_head(agent_id);
CREATE INDEX IF NOT EXISTS idx_h2h_opponent ON agent_head_to_head(opponent_id);
