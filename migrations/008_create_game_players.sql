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
