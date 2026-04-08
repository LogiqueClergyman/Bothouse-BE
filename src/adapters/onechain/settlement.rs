use async_trait::async_trait;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::game::WinnerEntry;
use crate::errors::AppError;
use crate::ports::settlement_port::SettlementPort;

pub struct OneChainSettlement {
    rpc_url: String,
    private_key: String,       // Ed25519 secret key hex (0x-prefixed, 64 hex chars)
    package_id: String,        // Move package ID (0x + 64 hex)
    house_wallet: String,      // House wallet address — rake recipient
}

impl OneChainSettlement {
    pub fn new(rpc_url: String, private_key: String, package_id: String, house_wallet: String) -> Self {
        Self { rpc_url, private_key, package_id, house_wallet }
    }

    /// Build an HTTP client for RPC calls (longer timeout for tx execution).
    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client")
    }

    /// Send a JSON-RPC 2.0 request and return the "result" field (with retries for transient errors).
    async fn rpc_call(&self, method: &str, params: Vec<Value>) -> Result<Value, AppError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let mut last_err = AppError::Internal(anyhow::anyhow!("RPC call failed"));
        for attempt in 0..3u32 {
            let resp = match self.client()
                .post(&self.rpc_url)
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_err = AppError::Internal(anyhow::anyhow!("RPC request failed: {}", e));
                    if attempt < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000 * 2u64.pow(attempt))).await;
                        continue;
                    }
                    return Err(last_err);
                }
            };

            let status = resp.status();
            if status.is_server_error() && attempt < 2 {
                last_err = AppError::Internal(anyhow::anyhow!("RPC HTTP {}", status));
                tokio::time::sleep(tokio::time::Duration::from_millis(1000 * 2u64.pow(attempt))).await;
                continue;
            }

            let json: Value = resp.json().await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("RPC response parse failed: {}", e)))?;

            if let Some(error) = json.get("error") {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "RPC error: {}", error
                )));
            }

            return json.get("result")
                .cloned()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("RPC response missing 'result'")));
        }

        Err(last_err)
    }

    /// Derive the Sui address from the Ed25519 signing key.
    /// Sui address = 0x + hex(blake2b_256(0x00 || public_key_bytes))[0..64]
    fn sender_address(&self) -> Result<String, AppError> {
        let signing_key = self.parse_signing_key()?;
        let pk_bytes = signing_key.verifying_key().to_bytes();
        // Sui Ed25519 flag = 0x00
        let mut hasher = blake2b_simd::Params::new().hash_length(32).to_state();
        hasher.update(&[0x00]);
        hasher.update(&pk_bytes);
        let hash = hasher.finalize();
        Ok(format!("0x{}", hex::encode(hash.as_bytes())))
    }

    fn parse_signing_key(&self) -> Result<SigningKey, AppError> {
        let hex_str = self.private_key.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid private key hex: {}", e)))?;
        let key_bytes: [u8; 32] = bytes.try_into()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Private key must be 32 bytes")))?;
        Ok(SigningKey::from_bytes(&key_bytes))
    }

    /// Find the Game shared object ID by querying GameCreated events for a matching game_uuid.
    async fn find_game_object_id(&self, game_id: Uuid) -> Result<String, AppError> {
        let game_uuid_bytes: Vec<u8> = game_id.as_bytes().to_vec();

        // Query events of type {package}::escrow::GameCreated
        let event_type = format!("{}::escrow::GameCreated", self.package_id);
        let result = self.rpc_call("suix_queryEvents", vec![
            json!({ "MoveEventType": event_type }),
            Value::Null,     // cursor
            json!(50),       // limit
            json!(true),     // descending (newest first)
        ]).await?;

        let events = result.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No event data in response")))?;

        for event in events {
            let parsed = event.get("parsedJson")
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Event missing parsedJson")))?;

            // game_uuid is stored as a vector of u8 numbers in the event
            if let Some(uuid_arr) = parsed.get("game_uuid").and_then(|v| v.as_array()) {
                let event_bytes: Vec<u8> = uuid_arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                if event_bytes == game_uuid_bytes {
                    // Found it — game_id in the event is the object ID
                    let game_object_id = parsed.get("game_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Event missing game_id")))?;
                    return Ok(game_object_id.to_string());
                }
            }
        }

        Err(AppError::Internal(anyhow::anyhow!(
            "Game object not found for game_id {}", game_id
        )))
    }

    /// Find the GameAdminCap object owned by the server wallet for this Game.
    async fn find_admin_cap(&self, game_object_id: &str) -> Result<String, AppError> {
        let sender = self.sender_address()?;
        let cap_type = format!("{}::escrow::GameAdminCap", self.package_id);

        let result = self.rpc_call("suix_getOwnedObjects", vec![
            json!(sender),
            json!({
                "filter": { "StructType": cap_type },
                "options": { "showContent": true },
            }),
            Value::Null,  // cursor
            json!(50),    // limit
        ]).await?;

        let objects = result.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No data in getOwnedObjects")))?;

        for obj in objects {
            let content = obj.get("data")
                .and_then(|d| d.get("content"))
                .and_then(|c| c.get("fields"));
            if let Some(fields) = content {
                // GameAdminCap.game_id is the object ID of the Game it administers
                if let Some(cap_game_id) = fields.get("game_id").and_then(|v| v.as_str()) {
                    if cap_game_id == game_object_id {
                        let cap_id = obj.get("data")
                            .and_then(|d| d.get("objectId"))
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Cap missing objectId")))?;
                        return Ok(cap_id.to_string());
                    }
                }
            }
        }

        Err(AppError::Internal(anyhow::anyhow!(
            "GameAdminCap not found for Game {}", game_object_id
        )))
    }

    /// Sign transaction bytes and execute via sui_executeTransactionBlock.
    /// `tx_bytes` is base64-encoded BCS TransactionData from unsafe_moveCall.
    async fn sign_and_execute(&self, tx_bytes_b64: &str) -> Result<String, AppError> {
        let signing_key = self.parse_signing_key()?;

        // Decode tx bytes
        let tx_bytes = base64::engine::general_purpose::STANDARD.decode(tx_bytes_b64)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Base64 decode failed: {}", e)))?;

        // Sui intent signing: intent_msg = [0, 0, 0] || bcs_tx_bytes
        // Then sign blake2b_256(intent_msg)
        let mut intent_msg = vec![0u8, 0, 0];
        intent_msg.extend_from_slice(&tx_bytes);

        let mut hasher = blake2b_simd::Params::new().hash_length(32).to_state();
        hasher.update(&intent_msg);
        let digest = hasher.finalize();

        let signature = signing_key.sign(digest.as_bytes());

        // Sui serialized signature: flag_byte || signature_bytes || public_key_bytes
        // Ed25519 flag = 0x00
        let pk_bytes = signing_key.verifying_key().to_bytes();
        let mut serialized_sig = Vec::with_capacity(1 + 64 + 32);
        serialized_sig.push(0x00); // Ed25519 flag
        serialized_sig.extend_from_slice(&signature.to_bytes());
        serialized_sig.extend_from_slice(&pk_bytes);

        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&serialized_sig);

        // Execute
        let result = self.rpc_call("sui_executeTransactionBlock", vec![
            json!(tx_bytes_b64),
            json!([sig_b64]),
            json!({
                "showEffects": true,
                "showEvents": true,
            }),
            json!("WaitForLocalExecution"),
        ]).await?;

        // Extract digest
        let digest_str = result.get("digest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!(
                "Transaction response missing digest: {:?}", result
            )))?;

        // Check execution status
        if let Some(effects) = result.get("effects") {
            if let Some(status) = effects.get("status").and_then(|s| s.get("status")).and_then(|s| s.as_str()) {
                if status != "success" {
                    let error_msg = effects.get("status")
                        .and_then(|s| s.get("error"))
                        .and_then(|e| e.as_str())
                        .unwrap_or("unknown error");
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "Transaction failed: {}", error_msg
                    )));
                }
            }
        }

        Ok(digest_str.to_string())
    }
}

#[async_trait]
impl SettlementPort for OneChainSettlement {
    async fn create_game(&self, game_id: Uuid, buy_in_atomic: &str) -> Result<String, AppError> {
        let sender = self.sender_address()?;

        // create_game(game_uuid: vector<u8>, buy_in: u64)
        let game_uuid_bytes: Vec<u8> = game_id.as_bytes().to_vec();
        let buy_in: u64 = buy_in_atomic
            .parse()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid buy_in_atomic: {}", e)))?;

        let tx_result = self
            .rpc_call(
                "unsafe_moveCall",
                vec![
                    json!(sender),              // signer
                    json!(self.package_id),     // package
                    json!("escrow"),            // module
                    json!("create_game"),       // function
                    json!([]),                  // type_arguments
                    // Sui JSON-RPC expects u64 arguments as strings
                    json!([game_uuid_bytes, buy_in.to_string()]),
                    Value::Null,                // gas (auto)
                    json!("50000000"),          // gas_budget
                ],
            )
            .await?;

        let tx_bytes = tx_result
            .get("txBytes")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "unsafe_moveCall missing txBytes: {:?}",
                    tx_result
                ))
            })?;

        self.sign_and_execute(tx_bytes).await
    }

    async fn start_game(&self, _game_id: Uuid) -> Result<String, AppError> {
        // OneChain Move escrow doesn't have a start_game step; game is "open" until settled/cancelled.
        Ok(String::new())
    }

    async fn settle(
        &self,
        game_id: Uuid,
        winners: &[WinnerEntry],
        rake_atomic: &str,
        result_hash: &str,
    ) -> Result<String, AppError> {
        let sender = self.sender_address()?;
        let game_object_id = self.find_game_object_id(game_id).await?;
        let admin_cap_id = self.find_admin_cap(&game_object_id).await?;

        // Build arguments for escrow::settle(cap, game, winners, payouts, rake, rake_recipient, result_hash)
        let winner_addrs: Vec<String> = winners.iter()
            .map(|w| w.wallet_address.clone())
            .collect();
        let payouts: Vec<String> = winners.iter()
            .map(|w| w.amount_won_atomic.clone())
            .collect();
        let rake: u64 = rake_atomic.parse()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid rake amount: {}", e)))?;

        // result_hash is hex string — convert to bytes for Move vector<u8>
        let result_hash_hex = result_hash.trim_start_matches("0x");
        let result_hash_bytes = hex::decode(result_hash_hex)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid result_hash hex: {}", e)))?;

        // Use unsafe_moveCall to build the transaction server-side
        // This avoids BCS serialization on our end — the RPC node builds the PTB
        let tx_result = self.rpc_call("unsafe_moveCall", vec![
            json!(sender),                                              // signer
            json!(self.package_id),                                     // package
            json!("escrow"),                                            // module
            json!("settle"),                                            // function
            json!([]),                                                  // type_arguments (none)
            json!([
                admin_cap_id,                                           // cap: &GameAdminCap
                game_object_id,                                         // game: &mut Game
                winner_addrs,                                           // winners: vector<address>
                payouts,                                                // payouts: vector<u64>
                rake,                                                   // rake: u64
                self.house_wallet,                                      // rake_recipient: address
                result_hash_bytes,                                      // result_hash: vector<u8>
            ]),
            Value::Null,                                                // gas (auto)
            json!("50000000"),                                          // gas_budget (50M MIST)
        ]).await?;

        // unsafe_moveCall returns { txBytes, gas, inputObjects }
        let tx_bytes = tx_result.get("txBytes")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!(
                "unsafe_moveCall missing txBytes: {:?}", tx_result
            )))?;

        // Sign and execute
        self.sign_and_execute(tx_bytes).await
    }

    async fn check_confirmation(&self, tx_hash: &str) -> Result<Option<i64>, AppError> {
        // OneChain (Sui) has BFT finality — if the tx exists and succeeded, it's final.
        let result = self.rpc_call("sui_getTransactionBlock", vec![
            json!(tx_hash),
            json!({
                "showEffects": true,
            }),
        ]).await;

        match result {
            Ok(tx) => {
                let status = tx.get("effects")
                    .and_then(|e| e.get("status"))
                    .and_then(|s| s.get("status"))
                    .and_then(|s| s.as_str());

                match status {
                    Some("success") => {
                        // Use checkpoint as "block number" equivalent
                        let checkpoint = tx.get("checkpoint")
                            .and_then(|c| c.as_str())
                            .and_then(|s| s.parse::<i64>().ok())
                            .or_else(|| {
                                tx.get("checkpoint")
                                    .and_then(|c| c.as_i64())
                            });
                        Ok(Some(checkpoint.unwrap_or(1)))
                    }
                    Some("failure") => Ok(None),
                    _ => Ok(None),
                }
            }
            Err(_) => {
                // Transaction not found yet
                Ok(None)
            }
        }
    }

    async fn check_escrow_deposit(
        &self,
        game_id: Uuid,
        wallet: &str,
        _buy_in_atomic: &str,
    ) -> Result<bool, AppError> {
        let game_object_id = self.find_game_object_id(game_id).await?;

        // Read the Game object with full content
        let result = self.rpc_call("sui_getObject", vec![
            json!(game_object_id),
            json!({
                "showContent": true,
            }),
        ]).await?;

        let fields = result.get("data")
            .and_then(|d| d.get("content"))
            .and_then(|c| c.get("fields"))
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!(
                "Game object {} has no content fields", game_object_id
            )))?;

        // depositors is a vector<address> — check if wallet is in it
        let empty = vec![];
        let depositors = fields.get("depositors")
            .and_then(|d| d.as_array())
            .unwrap_or(&empty);

        let wallet_lower = wallet.to_lowercase();
        let found = depositors.iter().any(|addr| {
            addr.as_str()
                .map(|a| a.to_lowercase() == wallet_lower)
                .unwrap_or(false)
        });

        Ok(found)
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
