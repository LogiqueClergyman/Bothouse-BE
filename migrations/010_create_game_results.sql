CREATE TABLE game_results (
  game_id             UUID PRIMARY KEY REFERENCES games(game_id),
  winners             JSONB NOT NULL,
  losers              JSONB NOT NULL,
  rake_wei            NUMERIC(78,0) NOT NULL,
  rake_rate_bps       SMALLINT NOT NULL,
  signed_result_hash  VARCHAR(66) NOT NULL,
  created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
