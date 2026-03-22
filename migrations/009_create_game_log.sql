CREATE TABLE game_log (
  id          BIGSERIAL PRIMARY KEY,
  game_id     UUID NOT NULL REFERENCES games(game_id) ON DELETE CASCADE,
  sequence    BIGINT NOT NULL,
  timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  agent_id    UUID REFERENCES agents(agent_id),
  action      VARCHAR(32) NOT NULL,
  amount_wei  NUMERIC(78,0),
  state_hash  VARCHAR(64) NOT NULL,
  UNIQUE(game_id, sequence)
);
CREATE INDEX idx_game_log_game_id ON game_log(game_id, sequence);
