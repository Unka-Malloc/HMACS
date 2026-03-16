use crate::chain::{ChainAdapter, DepositEvent};
use hmacs_core::{Chain, HmacsResult, SettlementId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepositStatus {
    Pending,
    Confirmed,
    Credited,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRecord {
    pub id: SettlementId,
    pub chain: Chain,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub token_address: Option<String>,
    pub amount: Decimal,
    pub status: DepositStatus,
    pub block_number: u64,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

impl DepositRecord {
    pub fn from_event(event: &DepositEvent) -> Self {
        Self {
            id: SettlementId::new(),
            chain: event.chain,
            tx_hash: event.tx_hash.clone(),
            from_address: event.from_address.clone(),
            to_address: event.to_address.clone(),
            token_address: event.token_address.clone(),
            amount: event.amount,
            status: DepositStatus::Pending,
            block_number: event.block_number,
            created_at: Utc::now(),
            confirmed_at: None,
        }
    }

    pub fn confirm(&mut self) {
        self.status = DepositStatus::Confirmed;
        self.confirmed_at = Some(Utc::now());
    }

    pub fn credit(&mut self) {
        self.status = DepositStatus::Credited;
    }
}

/// Scans for new deposits on-chain and returns deposit records.
pub async fn scan_deposits(
    adapter: Arc<dyn ChainAdapter>,
    watch_address: &str,
    token_address: Option<&str>,
    from_slot: Option<u64>,
) -> HmacsResult<Vec<DepositRecord>> {
    let events = adapter
        .watch_deposits(watch_address, token_address, from_slot)
        .await?;

    let records: Vec<DepositRecord> = events.iter().map(DepositRecord::from_event).collect();

    info!(
        chain = %adapter.chain(),
        address = watch_address,
        count = records.len(),
        "Scanned deposits"
    );

    Ok(records)
}
