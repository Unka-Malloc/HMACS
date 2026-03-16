use async_trait::async_trait;
use hmacs_core::{Chain, HmacsError, HmacsResult};
use rust_decimal::Decimal;
use tracing::info;

use crate::chain::{ChainAdapter, ChainTxResult, DepositEvent, OnChainBalance};

/// Solana blockchain adapter.
pub struct SolanaAdapter {
    rpc_url: String,
}

impl SolanaAdapter {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
        }
    }

    pub fn devnet() -> Self {
        Self::new("https://api.devnet.solana.com")
    }

    pub fn mainnet() -> Self {
        Self::new("https://api.mainnet-beta.solana.com")
    }
}

#[async_trait]
impl ChainAdapter for SolanaAdapter {
    fn chain(&self) -> Chain {
        Chain::Solana
    }

    async fn get_balance(
        &self,
        address: &str,
        token_address: Option<&str>,
    ) -> HmacsResult<OnChainBalance> {
        info!(
            chain = "solana",
            address = address,
            token = token_address,
            rpc = %self.rpc_url,
            "Querying balance"
        );

        // TODO: Implement using solana-client RpcClient
        // For now, return a placeholder that compiles.
        Ok(OnChainBalance {
            address: address.to_string(),
            chain: Chain::Solana,
            token_address: token_address.map(String::from),
            balance: Decimal::ZERO,
            decimals: if token_address.is_some() { 6 } else { 9 },
        })
    }

    async fn send_token(
        &self,
        _from_keypair: &[u8],
        to_address: &str,
        token_address: Option<&str>,
        amount: Decimal,
        _decimals: u8,
    ) -> HmacsResult<ChainTxResult> {
        info!(
            chain = "solana",
            to = to_address,
            token = token_address,
            amount = %amount,
            "Sending token"
        );

        // TODO: Build and send actual Solana transaction
        Err(HmacsError::SettlementError(
            "Solana send_token not yet implemented".into(),
        ))
    }

    async fn verify_transaction(&self, tx_hash: &str) -> HmacsResult<ChainTxResult> {
        info!(chain = "solana", tx_hash = tx_hash, "Verifying transaction");

        // TODO: Verify transaction via RPC
        Err(HmacsError::SettlementError(
            "Solana verify_transaction not yet implemented".into(),
        ))
    }

    async fn watch_deposits(
        &self,
        watch_address: &str,
        token_address: Option<&str>,
        _from_slot: Option<u64>,
    ) -> HmacsResult<Vec<DepositEvent>> {
        info!(
            chain = "solana",
            address = watch_address,
            token = token_address,
            "Watching for deposits"
        );

        // TODO: Query recent transactions for the watch address
        Ok(vec![])
    }
}
