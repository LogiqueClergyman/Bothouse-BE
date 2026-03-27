use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::game::WinnerEntry;
use crate::errors::AppError;
use crate::ports::settlement_port::SettlementPort;

pub struct OneChainSettlement {
    rpc_url: String,
    private_key: String,       // Ed25519 secret key hex (0x-prefixed)
    package_id: String,        // Move package ID (0x + 64 hex)
    house_wallet: String,      // House wallet address — used as rake_recipient in settle()
}

impl OneChainSettlement {
    pub fn new(rpc_url: String, private_key: String, package_id: String, house_wallet: String) -> Self {
        Self { rpc_url, private_key, package_id, house_wallet }
    }
}

#[async_trait]
impl SettlementPort for OneChainSettlement {
    async fn settle(
        &self,
        game_id: Uuid,
        winners: &[WinnerEntry],
        _rake_atomic: &str,
        result_hash: &str,
    ) -> Result<String, AppError> {
        // Build a ProgrammableTransactionBlock:
        //   bothouse::escrow::settle(game_shared_object, winners, amounts, result_hash)
        // Sign with Ed25519 keypair and execute via JSON-RPC.
        // Returns the base58 transaction digest.
        //
        // TODO: Implement using sui-sdk crate once added to Cargo.toml.
        let _ = (game_id, winners, result_hash, &self.rpc_url, &self.private_key, &self.package_id, &self.house_wallet);
        Err(AppError::Internal(anyhow::anyhow!(
            "OneChain settlement not yet implemented — add sui-sdk to Cargo.toml"
        )))
    }

    async fn check_confirmation(&self, tx_hash: &str) -> Result<Option<i64>, AppError> {
        // OneChain (Sui) has instant BFT finality.
        // Call sui_getTransactionBlock; if status is "success", return the checkpoint
        // number as block_number. If the TX is not found, return an error.
        //
        // Unlike EVM, there is no need to wait for confirmations.
        //
        // TODO: Implement using sui-sdk crate.
        let _ = (tx_hash, &self.rpc_url);
        Err(AppError::Internal(anyhow::anyhow!(
            "OneChain confirmation check not yet implemented"
        )))
    }

    async fn check_escrow_deposit(
        &self,
        game_id: Uuid,
        wallet: &str,
        _buy_in_atomic: &str,
    ) -> Result<bool, AppError> {
        // 1. Find the Game shared object by game_id bytes
        // 2. Read the `deposits` VecMap field via sui_getObject with showContent: true
        // 3. Check if wallet address is a key in the map
        //
        // TODO: Implement using sui-sdk crate.
        let _ = (game_id, wallet, &self.rpc_url, &self.package_id);
        Err(AppError::Internal(anyhow::anyhow!(
            "OneChain escrow deposit check not yet implemented"
        )))
    }
}

/// Verify an Ed25519 signature against a known public key.
/// Unlike EVM ecrecover, Ed25519 cannot recover the signer — we must verify
/// against the agent's registered public key.
pub fn verify_ed25519_signature(
    message: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<bool, anyhow::Error> {
    use ed25519_dalek::{Signature, VerifyingKey, Verifier};
    use sha2::{Sha256, Digest};

    let sig_bytes = hex::decode(signature_hex.trim_start_matches("0x"))?;
    let sig = Signature::from_slice(&sig_bytes)?;

    let pk_bytes = hex::decode(public_key_hex.trim_start_matches("0x"))?;
    let vk = VerifyingKey::from_bytes(
        pk_bytes.as_slice().try_into().map_err(|_| anyhow::anyhow!("invalid public key length"))?,
    )?;

    // OneChain uses sha256 of the raw message bytes as the digest
    let digest = Sha256::digest(message);

    Ok(vk.verify(&digest, &sig).is_ok())
}
