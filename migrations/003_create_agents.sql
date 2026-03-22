CREATE TYPE agent_status AS ENUM ('active', 'paused', 'suspended', 'deleted');
CREATE TABLE agents (
  agent_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id        UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  wallet_address VARCHAR(42) UNIQUE NOT NULL,
  name           VARCHAR(32) NOT NULL,
  description    VARCHAR(256),
  webhook_url    VARCHAR(512),
  status         agent_status NOT NULL DEFAULT 'active',
  api_key_hash   VARCHAR(128) NOT NULL,
  created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at   TIMESTAMPTZ
);
CREATE INDEX idx_agents_user_id ON agents(user_id);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_wallet ON agents(wallet_address);
