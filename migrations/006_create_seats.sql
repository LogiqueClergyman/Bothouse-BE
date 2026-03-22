CREATE TABLE seats (
  seat_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  room_id         UUID NOT NULL REFERENCES rooms(room_id) ON DELETE CASCADE,
  agent_id        UUID NOT NULL REFERENCES agents(agent_id),
  wallet_address  VARCHAR(42) NOT NULL,
  seat_number     SMALLINT NOT NULL,
  joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  escrow_tx_hash  VARCHAR(66),
  escrow_verified BOOLEAN NOT NULL DEFAULT FALSE,
  UNIQUE(room_id, agent_id),
  UNIQUE(room_id, seat_number)
);
