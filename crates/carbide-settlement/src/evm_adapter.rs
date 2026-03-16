use async_trait::async_trait;
use carbide_core::{Chain, CarbideError, CarbideResult};
use rust_decimal::Decimal;
use tracing::info;

use crate::chain::{ChainAdapter, ChainTxResult, DepositEvent, OnChainBalance};

/// EVM-compatible blockchain adapter (Ethereum, Polygon, Arbitrum, Base, etc.).
pub struct EvmAdapter {
    chain: Chain,
    rpc_url: String,
}

impl EvmAdapter {
    pub fn new(chain: Chain, rpc_url: &str) -> Self {
        Self {
            chain,
            rpc_url: rpc_url.to_string(),
        }
    }

    pub fn ethereum_mainnet() -> Self {
        Self::new(Chain::Ethereum, "https://eth.llamarpc.com")
    }

    pub fn ethereum_sepolia() -> Self {
        Self::new(Chain::Ethereum, "https://rpc.sepolia.org")
    }
}

#[async_trait]
impl ChainAdapter for EvmAdapter {
    fn chain(&self) -> Chain {
        self.chain
    }

    async fn get_balance(
        &self,
        address: &str,
        token_address: Option<&str>,
    ) -> CarbideResult<OnChainBalance> {
        info!(
            chain = %self.chain,
            address = address,
            token = token_address,
            rpc = %self.rpc_url,
            "Querying EVM balance"
        );

        // TODO: Implement using alloy provider
        Ok(OnChainBalance {
            address: address.to_string(),
            chain: self.chain,
            token_address: token_address.map(String::from),
            balance: Decimal::ZERO,
            decimals: if token_address.is_some() { 6 } else { 18 },
        })
    }

    async fn send_token(
        &self,
        _from_keypair: &[u8],
        to_address: &str,
        token_address: Option<&str>,
        amount: Decimal,
        _decimals: u8,
    ) -> CarbideResult<ChainTxResult> {
        info!(
            chain = %self.chain,
            to = to_address,
            token = token_address,
            amount = %amount,
            "Sending EVM token"
        );

        // TODO: Build and send EVM transaction via alloy
        Err(CarbideError::SettlementError(
            "EVM send_token not yet implemented".into(),
        ))
    }

    async fn verify_transaction(&self, tx_hash: &str) -> CarbideResult<ChainTxResult> {
        info!(
            chain = %self.chain,
            tx_hash = tx_hash,
            "Verifying EVM transaction"
        );

        // TODO: Get transaction receipt via alloy
        Err(CarbideError::SettlementError(
            "EVM verify_transaction not yet implemented".into(),
        ))
    }

    async fn watch_deposits(
        &self,
        watch_address: &str,
        token_address: Option<&str>,
        _from_slot: Option<u64>,
    ) -> CarbideResult<Vec<DepositEvent>> {
        info!(
            chain = %self.chain,
            address = watch_address,
            token = token_address,
            "Watching for EVM deposits"
        );

        // TODO: Query Transfer events
        Ok(vec![])
    }
}
