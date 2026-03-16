use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, ParticipantId, TransactionId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryType {
    Deposit,
    Withdrawal,
    Freeze,
    Unfreeze,
    Transfer,
    Fee,
    Refund,
    Settlement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: TransactionId,
    pub participant_id: ParticipantId,
    pub entry_type: LedgerEntryType,
    pub asset: AssetSymbol,
    pub amount: Decimal,
    /// Positive = credit, Negative = debit
    pub direction: i8,
    /// Running balance after this entry
    pub balance_after: Decimal,
    /// Optional reference to another entity (task_id, order_id, settlement_id)
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}
