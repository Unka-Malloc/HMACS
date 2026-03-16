use crate::chain::ChainAdapter;
use hmacs_core::{Chain, HmacsResult};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub chain: Chain,
    pub address: String,
    pub token_address: Option<String>,
    pub on_chain_balance: Decimal,
    pub internal_balance: Decimal,
    pub discrepancy: Decimal,
    pub is_balanced: bool,
    pub checked_at: DateTime<Utc>,
}

/// Reconcile on-chain balance with internal ledger balance.
pub async fn reconcile(
    adapter: Arc<dyn ChainAdapter>,
    address: &str,
    token_address: Option<&str>,
    internal_balance: Decimal,
) -> HmacsResult<ReconciliationReport> {
    let on_chain = adapter.get_balance(address, token_address).await?;

    let discrepancy = on_chain.balance - internal_balance;
    let is_balanced = discrepancy.abs() < Decimal::new(1, 6); // tolerance of 0.000001

    if !is_balanced {
        warn!(
            chain = %adapter.chain(),
            address = address,
            on_chain = %on_chain.balance,
            internal = %internal_balance,
            discrepancy = %discrepancy,
            "Balance discrepancy detected"
        );
    } else {
        info!(
            chain = %adapter.chain(),
            address = address,
            balance = %on_chain.balance,
            "Reconciliation passed"
        );
    }

    Ok(ReconciliationReport {
        chain: adapter.chain(),
        address: address.to_string(),
        token_address: token_address.map(String::from),
        on_chain_balance: on_chain.balance,
        internal_balance,
        discrepancy,
        is_balanced,
        checked_at: Utc::now(),
    })
}
