use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_secs: u64,
    pub refresh_token_expiry_secs: u64,
    pub bcrypt_cost: u32,
    pub house_signing_key: String,
    pub turn_timeout_ms: u64,
    pub chain_type: String,
    pub settlement_rpc_url: String,
    pub settlement_private_key: String,
    pub escrow_contract_address: String,
    pub house_wallet_address: String,
    pub chain_id: u64,
    pub rake_bps: u16,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub base_url: String,
    pub testnet_base_url: String,
    pub skip_escrow_verification: bool,
    pub skip_action_signature_verification: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        fn parse_bool_env(key: &str, default: bool) -> bool {
            match std::env::var(key) {
                Ok(v) => matches!(
                    v.trim().to_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "on"
                ),
                Err(_) => default,
            }
        }

        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            redis_url: std::env::var("REDIS_URL")
                .context("REDIS_URL must be set")?,
            jwt_secret: std::env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,
            jwt_expiry_secs: std::env::var("JWT_EXPIRY_SECS")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .context("JWT_EXPIRY_SECS must be a number")?,
            refresh_token_expiry_secs: std::env::var("REFRESH_TOKEN_EXPIRY_SECS")
                .unwrap_or_else(|_| "2592000".to_string())
                .parse()
                .context("REFRESH_TOKEN_EXPIRY_SECS must be a number")?,
            bcrypt_cost: std::env::var("BCRYPT_COST")
                .unwrap_or_else(|_| "12".to_string())
                .parse()
                .context("BCRYPT_COST must be a number")?,
            house_signing_key: std::env::var("HOUSE_SIGNING_KEY")
                .context("HOUSE_SIGNING_KEY must be set")?,
            turn_timeout_ms: std::env::var("TURN_TIMEOUT_MS")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()
                .context("TURN_TIMEOUT_MS must be a number")?,
            chain_type: std::env::var("CHAIN_TYPE")
                .unwrap_or_else(|_| "evm".to_string()),
            settlement_rpc_url: std::env::var("SETTLEMENT_RPC_URL")
                .context("SETTLEMENT_RPC_URL must be set")?,
            settlement_private_key: std::env::var("SETTLEMENT_PRIVATE_KEY")
                .context("SETTLEMENT_PRIVATE_KEY must be set")?,
            escrow_contract_address: std::env::var("ESCROW_CONTRACT_ADDRESS")
                .context("ESCROW_CONTRACT_ADDRESS must be set")?,
            house_wallet_address: std::env::var("HOUSE_WALLET_ADDRESS")
                .context("HOUSE_WALLET_ADDRESS must be set")?,
            chain_id: std::env::var("CHAIN_ID")
                .unwrap_or_else(|_| "8453".to_string())
                .parse()
                .context("CHAIN_ID must be a number")?,
            rake_bps: std::env::var("RAKE_BPS")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .context("RAKE_BPS must be a number")?,
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("PORT must be a number")?,
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            base_url: std::env::var("BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            testnet_base_url: std::env::var("TESTNET_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            skip_escrow_verification: parse_bool_env("SKIP_ESCROW_VERIFICATION", false),
            skip_action_signature_verification: parse_bool_env(
                "SKIP_ACTION_SIGNATURE_VERIFICATION",
                false,
            ),
        })
    }
}
