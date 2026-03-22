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
