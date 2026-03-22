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
