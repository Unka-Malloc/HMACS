use crate::chain::ChainAdapter;
use hmacs_core::{Chain, HmacsResult, SettlementId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WithdrawStatus {
    Pending,
    Processing,
    Submitted,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawRequest {
    pub id: SettlementId,
    pub chain: Chain,
    pub to_address: String,
    pub token_address: Option<String>,
    pub amount: Decimal,
    pub decimals: u8,
    pub status: WithdrawStatus,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl WithdrawRequest {
    pub fn new(
        chain: Chain,
        to_address: String,
        token_address: Option<String>,
        amount: Decimal,
        decimals: u8,
    ) -> Self {
        Self {
            id: SettlementId::new(),
            chain,
            to_address,
            token_address,
            amount,
            decimals,
            status: WithdrawStatus::Pending,
            tx_hash: None,
            created_at: Utc::now(),
            submitted_at: None,
            confirmed_at: None,
            error_message: None,
        }
    }
}

/// Process a withdrawal: send tokens on-chain.
pub async fn process_withdrawal(
    adapter: Arc<dyn ChainAdapter>,
    request: &mut WithdrawRequest,
    hot_wallet_keypair: &[u8],
) -> HmacsResult<()> {
    request.status = WithdrawStatus::Processing;

    info!(
        id = %request.id,
        chain = %request.chain,
        to = %request.to_address,
        amount = %request.amount,
        "Processing withdrawal"
    );

    match adapter
        .send_token(
            hot_wallet_keypair,
            &request.to_address,
            request.token_address.as_deref(),
            request.amount,
            request.decimals,
        )
        .await
    {
        Ok(result) => {
            request.tx_hash = Some(result.tx_hash);
            request.status = WithdrawStatus::Submitted;
            request.submitted_at = Some(Utc::now());

            if result.confirmed {
                request.status = WithdrawStatus::Confirmed;
                request.confirmed_at = Some(Utc::now());
            }

            Ok(())
        }
        Err(e) => {
            request.status = WithdrawStatus::Failed;
            request.error_message = Some(e.to_string());
            Err(e)
        }
    }
}
