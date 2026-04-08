use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;

use crate::domain::auth::Claims;
use crate::errors::AppError;
use crate::state::AppState;

fn is_valid_wallet_address(addr: &str, chain_type: &str) -> bool {
    match chain_type {
        "evm" => {
            // 0x + 40 hex chars
            addr.len() == 42
                && addr.starts_with("0x")
                && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
        }
        "onechain" => {
            // 0x + 64 hex chars (32-byte Sui address)
            addr.len() == 66
                && addr.starts_with("0x")
                && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
        }
        _ => false,
    }
}

pub async fn generate_nonce(
    wallet: &str,
    state: &AppState,
) -> Result<(String, chrono::DateTime<Utc>), AppError> {
    if !is_valid_wallet_address(wallet, &state.config.chain_type) {
        return Err(AppError::BadRequest("Invalid wallet address".to_string()));
    }
    let nonce = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    };
    let expires_at = Utc::now() + chrono::Duration::seconds(300);
    state
        .cache
        .set_nonce(&wallet.to_lowercase(), &nonce)
        .await?;
    Ok((nonce, expires_at))
}

pub async fn verify_signature(
    wallet: &str,
    signature: &str,
    state: &AppState,
) -> Result<(String, String), AppError> {
    let wallet_lower = wallet.to_lowercase();

    let nonce = state
        .cache
        .get_nonce(&wallet_lower)
        .await?
        .ok_or_else(|| AppError::Unauthorized("NONCE_EXPIRED".to_string()))?;

    // Verify signature — dispatch based on chain type
    let valid = match state.config.chain_type.as_str() {
        "evm" => {
            let recovered = recover_signer(&nonce, signature)
                .map_err(|_| AppError::Unauthorized("INVALID_SIGNATURE".to_string()))?;
            recovered.to_lowercase() == wallet_lower
        }
        "onechain" => {
            // Ed25519: look up the agent's registered public key and verify.
            // The agent must have registered first; their public key is derived from the address
            // and stored at registration time.
            // For nonce auth, we verify the signature over the nonce bytes using the stored pubkey.
            verify_onechain_signature(&nonce, signature, &wallet_lower, state).await
                .map_err(|_| AppError::Unauthorized("INVALID_SIGNATURE".to_string()))?
        }
        _ => return Err(AppError::Unauthorized("UNSUPPORTED_CHAIN".to_string())),
    };

    if !valid {
        return Err(AppError::Unauthorized("INVALID_SIGNATURE".to_string()));
    }

    state.cache.delete_nonce(&wallet_lower).await?;

    let user = state.auth_store.upsert_user(&wallet_lower).await?;

    let refresh_token = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    };

    let expires_at = Utc::now()
        + chrono::Duration::seconds(state.config.refresh_token_expiry_secs as i64);
    let session = state
        .auth_store
        .create_session(user.user_id, &refresh_token, expires_at)
        .await?;

    let access_token = issue_jwt(&user.user_id, &wallet_lower, &session.session_id, state)?;

    state
        .cache
        .set_session(
            &session.session_id.to_string(),
            &user.user_id.to_string(),
            state.config.jwt_expiry_secs,
        )
        .await?;

    Ok((access_token, refresh_token))
}

pub async fn refresh_token(refresh_token: &str, state: &AppState) -> Result<String, AppError> {
    let session = state
        .auth_store
        .get_session_by_refresh_token(refresh_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("INVALID_REFRESH_TOKEN".to_string()))?;

    if session.revoked || session.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("INVALID_REFRESH_TOKEN".to_string()));
    }

    let user = state
        .auth_store
        .get_user_by_id(session.user_id)
        .await?
        .ok_or(AppError::NotFound)?;

    issue_jwt(&user.user_id, &user.wallet, &session.session_id, state)
}

pub async fn logout(session_id: Uuid, state: &AppState) -> Result<(), AppError> {
    state.auth_store.revoke_session(session_id).await?;
    state.cache.delete_session(&session_id.to_string()).await?;
    Ok(())
}

pub fn issue_jwt(
    user_id: &Uuid,
    wallet: &str,
    session_id: &Uuid,
    state: &AppState,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        wallet: wallet.to_string(),
        session_id: session_id.to_string(),
        iat: now,
        exp: now + state.config.jwt_expiry_secs as i64,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.into()))
}

async fn verify_onechain_signature(
    nonce: &str,
    signature: &str,
    wallet: &str,
    state: &AppState,
) -> Result<bool, anyhow::Error> {
    // Look up stored public key for this wallet from the auth store.
    // The public key must have been stored when the agent registered their wallet.
    let public_key_hex = state
        .auth_store
        .get_public_key(wallet)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No public key registered for wallet {}", wallet))?;

    // For nonce auth, verify the Ed25519 signature directly over raw nonce bytes
    // (no sha256 pre-hash — unlike game action signing which pre-hashes structured payloads).
    use ed25519_dalek::{Signature, VerifyingKey, Verifier};

    let sig_bytes = hex::decode(signature.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("invalid signature hex: {}", e))?;
    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|e| anyhow::anyhow!("invalid signature: {}", e))?;

    let pk_bytes = hex::decode(public_key_hex.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("invalid public key hex: {}", e))?;
    let vk = VerifyingKey::from_bytes(
        pk_bytes.as_slice().try_into().map_err(|_| anyhow::anyhow!("invalid public key length"))?,
    )?;

    Ok(vk.verify(nonce.as_bytes(), &sig).is_ok())
}

fn recover_signer(message: &str, signature: &str) -> Result<String, anyhow::Error> {
    use alloy::primitives::Signature;
    use alloy::signers::SignerSync;

    // EIP-191 personal_sign message construction
    let prefixed = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);
    let msg_hash = alloy::primitives::keccak256(prefixed.as_bytes());

    let sig: Signature = signature.parse()?;
    let recovered = sig.recover_address_from_prehash(&alloy::primitives::B256::from(msg_hash))?;
    Ok(format!("{:#x}", recovered))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::auth_store::MemoryAuthStore;
    use crate::adapters::memory::cache_store::MemoryCacheStore;
    use crate::adapters::memory::agent_store::MemoryAgentStore;
    use crate::adapters::memory::analytics_store::MemoryAnalyticsStore;
    use crate::adapters::memory::lobby_store::MemoryLobbyStore;
    use crate::adapters::memory::game_store::MemoryGameStore;
    use crate::adapters::memory::event_bus::MemoryEventBus;
    use crate::config::Config;
    use crate::games::GameRegistry;
    use std::sync::Arc;
    use tokio::sync::watch;

    fn make_state() -> AppState {
        let config = Config {
            database_url: "".to_string(),
            redis_url: "".to_string(),
            jwt_secret: "test_secret_key".to_string(),
            jwt_expiry_secs: 86400,
            refresh_token_expiry_secs: 2592000,
            bcrypt_cost: 4,
            house_signing_key: "deadbeef".to_string(),
            turn_timeout_ms: 10000,
            chain_type: "evm".to_string(),
            settlement_rpc_url: "".to_string(),
            settlement_private_key: "".to_string(),
            escrow_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
            house_wallet_address: "0x0000000000000000000000000000000000000000".to_string(),
            chain_id: 84532,
            rake_bps: 500,
            port: 8080,
            cors_origins: vec![],
            base_url: "http://localhost:8080".to_string(),
            testnet_base_url: "http://localhost:8080".to_string(),
            skip_escrow_verification: false,
            skip_action_signature_verification: false,
        };

        struct NoopSettlement;
        #[async_trait::async_trait]
        impl crate::ports::settlement_port::SettlementPort for NoopSettlement {
            async fn create_game(&self, _: Uuid, _: &str) -> Result<String, AppError> { Ok("0x".to_string()) }
            async fn start_game(&self, _: Uuid) -> Result<String, AppError> { Ok("0x".to_string()) }
            async fn settle(&self, _: Uuid, _: &[crate::domain::game::WinnerEntry], _: &str, _: &str) -> Result<String, AppError> { Ok("0x".to_string()) }
            async fn check_confirmation(&self, _: &str) -> Result<Option<i64>, AppError> { Ok(None) }
            async fn check_escrow_deposit(&self, _: Uuid, _: &str, _: &str) -> Result<bool, AppError> { Ok(true) }
        }

        struct NoopHttpClient;
        #[async_trait::async_trait]
        impl crate::ports::http_client::HttpClient for NoopHttpClient {
            async fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<u16, AppError> { Ok(200) }
        }

        let (shutdown_tx, _) = watch::channel(());

        AppState {
            auth_store: Arc::new(MemoryAuthStore::new()),
            agent_store: Arc::new(MemoryAgentStore::new()),
            analytics_store: Arc::new(MemoryAnalyticsStore::new()),
            game_store: Arc::new(MemoryGameStore::new()),
            lobby_store: Arc::new(MemoryLobbyStore::new()),
            cache: Arc::new(MemoryCacheStore::new()),
            event_bus: Arc::new(MemoryEventBus::new()),
            settlement: Arc::new(NoopSettlement),
            http_client: Arc::new(NoopHttpClient),
            game_registry: Arc::new(GameRegistry::new()),
            config: Arc::new(config),
            shutdown_tx: Arc::new(shutdown_tx),
        }
    }

    #[tokio::test]
    async fn test_generate_nonce_valid_wallet() {
        let state = make_state();
        let wallet = "0xabcdef1234567890abcdef1234567890abcdef12";
        let (nonce, expires_at) = generate_nonce(wallet, &state).await.unwrap();
        assert_eq!(nonce.len(), 64);
        assert!(expires_at > Utc::now());
    }

    #[tokio::test]
    async fn test_generate_nonce_invalid_wallet() {
        let state = make_state();
        let result = generate_nonce("notawallet", &state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nonce_expiry_removes_nonce() {
        let state = make_state();
        let wallet = "0xabcdef1234567890abcdef1234567890abcdef12";
        generate_nonce(wallet, &state).await.unwrap();
        // Nonce should be there
        let nonce = state.cache.get_nonce(&wallet.to_lowercase()).await.unwrap();
        assert!(nonce.is_some());
        // Delete it
        state.cache.delete_nonce(&wallet.to_lowercase()).await.unwrap();
        let nonce = state.cache.get_nonce(&wallet.to_lowercase()).await.unwrap();
        assert!(nonce.is_none());
    }

    #[tokio::test]
    async fn test_verify_signature_missing_nonce() {
        let state = make_state();
        let result = verify_signature(
            "0xabcdef1234567890abcdef1234567890abcdef12",
            "0x1234",
            &state,
        )
        .await;
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }
}
