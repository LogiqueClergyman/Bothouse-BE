use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::game::WinnerEntry;
use crate::errors::AppError;
use crate::ports::settlement_port::SettlementPort;

pub struct EthereumSettlement {
    rpc_url: String,
    private_key: String,
    contract_address: String,
    chain_id: u64,
}

impl EthereumSettlement {
    pub fn new(
        rpc_url: String,
        private_key: String,
        contract_address: String,
        chain_id: u64,
    ) -> Self {
        Self {
            rpc_url,
            private_key,
            contract_address,
            chain_id,
        }
    }
}

fn parse_address(s: &str) -> Result<alloy::primitives::Address, AppError> {
    s.parse::<alloy::primitives::Address>()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid address {}: {}", s, e)))
}

#[async_trait]
impl SettlementPort for EthereumSettlement {
    async fn settle(
        &self,
        game_id: Uuid,
        winners: &[WinnerEntry],
        _rake_wei: &str,
        result_hash: &str,
    ) -> Result<String, AppError> {
        use alloy::primitives::{Address, B256, U256};
        use alloy::providers::{Provider, ProviderBuilder};
        use alloy::signers::local::PrivateKeySigner;
        use alloy::network::EthereumWallet;
        use alloy::sol;
        use std::str::FromStr;

        sol!(
            #[allow(missing_docs)]
            #[sol(rpc)]
            BotTheHouseEscrow,
            r#"[{
                "name": "settle",
                "type": "function",
                "inputs": [
                    {"name": "gameId", "type": "bytes32"},
                    {"name": "winners", "type": "address[]"},
                    {"name": "amounts", "type": "uint256[]"},
                    {"name": "resultHash", "type": "bytes32"}
                ],
                "outputs": []
            }]"#
        );

        let signer: PrivateKeySigner = self
            .private_key
            .parse()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid private key")))?;
        let wallet = EthereumWallet::from(signer);

        let rpc_url = self.rpc_url.parse::<url::Url>()
            .map_err(|e| AppError::Internal(e.into()))?;

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url);

        let contract_addr = parse_address(&self.contract_address)?;
        let contract = BotTheHouseEscrow::new(contract_addr, provider);

        let game_id_bytes: B256 = {
            let bytes = game_id.as_bytes();
            let mut b = [0u8; 32];
            b[..16].copy_from_slice(bytes);
            B256::from(b)
        };

        let winner_addrs: Vec<Address> = winners
            .iter()
            .map(|w| parse_address(&w.wallet_address))
            .collect::<Result<_, _>>()?;

        let amounts: Vec<U256> = winners
            .iter()
            .map(|w| {
                U256::from_str(&w.amount_won_wei)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))
            })
            .collect::<Result<_, _>>()?;

        let result_hash_str = result_hash.trim_start_matches("0x");
        let mut rh_bytes = [0u8; 32];
        let decoded = hex::decode(result_hash_str).map_err(|e| AppError::Internal(e.into()))?;
        let len = decoded.len().min(32);
        rh_bytes[..len].copy_from_slice(&decoded[..len]);
        let result_hash_bytes = B256::from(rh_bytes);

        let call = contract.settle(game_id_bytes, winner_addrs, amounts, result_hash_bytes);
        let pending = call
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("settle tx: {}", e)))?;

        let tx_hash = format!("{:#x}", pending.tx_hash());
        Ok(tx_hash)
    }

    async fn check_confirmation(&self, tx_hash: &str) -> Result<Option<i64>, AppError> {
        use alloy::providers::{Provider, ProviderBuilder};
        use alloy::primitives::TxHash;

        let rpc_url = self.rpc_url.parse::<url::Url>()
            .map_err(|e| AppError::Internal(e.into()))?;
        let provider = ProviderBuilder::new().on_http(rpc_url);

        let hash: TxHash = tx_hash
            .parse()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid tx hash")))?;

        match provider.get_transaction_receipt(hash).await
            .map_err(|e| AppError::Internal(e.into()))? {
            None => Ok(None),
            Some(receipt) => {
                if receipt.status() {
                    Ok(receipt.block_number.map(|n| n as i64))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn check_escrow_deposit(
        &self,
        game_id: Uuid,
        wallet: &str,
        _buy_in_wei: &str,
    ) -> Result<bool, AppError> {
        use alloy::providers::{Provider, ProviderBuilder};
        use alloy::primitives::{Address, B256};
        use alloy::sol;

        sol!(
            #[allow(missing_docs)]
            #[sol(rpc)]
            EscrowView,
            r#"[{
                "name": "hasDeposited",
                "type": "function",
                "stateMutability": "view",
                "inputs": [
                    {"name": "gameId", "type": "bytes32"},
                    {"name": "player", "type": "address"}
                ],
                "outputs": [{"name": "", "type": "bool"}]
            }]"#
        );

        let rpc_url = self.rpc_url.parse::<url::Url>()
            .map_err(|e| AppError::Internal(e.into()))?;
        let provider = ProviderBuilder::new().on_http(rpc_url);

        let contract_addr = parse_address(&self.contract_address)?;
        let contract = EscrowView::new(contract_addr, provider);

        let game_id_bytes: B256 = {
            let bytes = game_id.as_bytes();
            let mut b = [0u8; 32];
            b[..16].copy_from_slice(bytes);
            B256::from(b)
        };

        let player_addr = parse_address(wallet)?;

        let result = contract
            .hasDeposited(game_id_bytes, player_addr)
            .call()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("hasDeposited: {}", e)))?;

        Ok(result._0)
    }
}
