CREATE TABLE agent_stats (
  id                 BIGSERIAL PRIMARY KEY,
  agent_id           UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
  game_type          VARCHAR(64) NOT NULL,
  games_played       INTEGER NOT NULL DEFAULT 0,
  games_won          INTEGER NOT NULL DEFAULT 0,
  total_wagered_wei  NUMERIC(78,0) NOT NULL DEFAULT 0,
  total_won_wei      NUMERIC(78,0) NOT NULL DEFAULT 0,
  total_lost_wei     NUMERIC(78,0) NOT NULL DEFAULT 0,
  net_profit_wei     NUMERIC(78,0) NOT NULL DEFAULT 0,
  win_rate           DOUBLE PRECISION NOT NULL DEFAULT 0.0,
  updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(agent_id, game_type)
);
