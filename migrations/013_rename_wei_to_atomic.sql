-- Migration 013: Rename _wei columns to _atomic (chain-agnostic naming)
-- and widen wallet/tx_hash columns for OneChain support.

-- ─── Widen wallet address columns ────────────────────────────────────────────
-- EVM: 42 chars (0x + 40 hex)
-- OneChain: 66 chars (0x + 64 hex)
ALTER TABLE users ALTER COLUMN wallet TYPE VARCHAR(66);
ALTER TABLE agents ALTER COLUMN wallet_address TYPE VARCHAR(66);
ALTER TABLE seats ALTER COLUMN wallet_address TYPE VARCHAR(66);
ALTER TABLE game_players ALTER COLUMN wallet_address TYPE VARCHAR(66);

-- ─── Widen tx hash columns ────────────────────────────────────────────────────
-- EVM: 66 chars (0x + 64 hex)
-- OneChain: base58 digest (~44 chars, but up to 88 to be safe)
ALTER TABLE seats ALTER COLUMN escrow_tx_hash TYPE VARCHAR(88);
ALTER TABLE settlements ALTER COLUMN tx_hash TYPE VARCHAR(88);
ALTER TABLE game_results ALTER COLUMN signed_result_hash TYPE VARCHAR(88);

-- ─── Rename _wei columns to _atomic ─────────────────────────────────────────
ALTER TABLE rooms RENAME COLUMN buy_in_wei TO buy_in_atomic;
ALTER TABLE game_players RENAME COLUMN stack_wei TO stack_atomic;
ALTER TABLE game_log RENAME COLUMN amount_wei TO amount_atomic;
ALTER TABLE game_results RENAME COLUMN rake_wei TO rake_atomic;
ALTER TABLE agent_stats RENAME COLUMN total_wagered_wei TO total_wagered_atomic;
ALTER TABLE agent_stats RENAME COLUMN total_won_wei TO total_won_atomic;
ALTER TABLE agent_stats RENAME COLUMN total_lost_wei TO total_lost_atomic;
ALTER TABLE agent_stats RENAME COLUMN net_profit_wei TO net_profit_atomic;
ALTER TABLE agent_head_to_head RENAME COLUMN agent_net_profit_wei TO agent_net_profit_atomic;

-- ─── New table: wallet_public_keys (for OneChain Ed25519 verification) ───────
-- Stores the Ed25519 public key for each OneChain wallet address.
-- EVM deployments will not populate this table.
CREATE TABLE IF NOT EXISTS wallet_public_keys (
    wallet          VARCHAR(66) PRIMARY KEY,
    public_key_hex  VARCHAR(64) NOT NULL,  -- 32-byte Ed25519 public key as hex
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
