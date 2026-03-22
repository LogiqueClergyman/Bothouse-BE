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
