use async_trait::async_trait;
use carbide_core::{Chain, CarbideResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Unified result from a blockchain transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTxResult {
    pub tx_hash: String,
    pub chain: Chain,
    pub confirmed: bool,
    pub block_number: Option<u64>,
}

/// Information about a token balance on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainBalance {
    pub address: String,
    pub chain: Chain,
    pub token_address: Option<String>,
    pub balance: Decimal,
    pub decimals: u8,
}

/// Deposit event detected on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositEvent {
    pub tx_hash: String,
    pub chain: Chain,
    pub from_address: String,
    pub to_address: String,
    pub token_address: Option<String>,
    pub amount: Decimal,
    pub block_number: u64,
}

/// The chain adapter trait that all blockchain implementations must satisfy.
#[async_trait]
pub trait ChainAdapter: Send + Sync {
    fn chain(&self) -> Chain;

    async fn get_balance(
        &self,
        address: &str,
        token_address: Option<&str>,
    ) -> CarbideResult<OnChainBalance>;

    async fn send_token(
        &self,
        from_keypair: &[u8],
        to_address: &str,
        token_address: Option<&str>,
        amount: Decimal,
        decimals: u8,
    ) -> CarbideResult<ChainTxResult>;

    async fn verify_transaction(&self, tx_hash: &str) -> CarbideResult<ChainTxResult>;

    async fn watch_deposits(
        &self,
        watch_address: &str,
        token_address: Option<&str>,
        from_slot: Option<u64>,
    ) -> CarbideResult<Vec<DepositEvent>>;
}
