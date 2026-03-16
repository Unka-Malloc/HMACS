use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, ParticipantId, WalletId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub wallet_id: WalletId,
    pub participant_id: ParticipantId,
    pub asset: AssetSymbol,
    pub available: Decimal,
    pub frozen: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl Balance {
    pub fn total(&self) -> Decimal {
        self.available + self.frozen
    }

    pub fn can_spend(&self, amount: Decimal) -> bool {
        self.available >= amount
    }

    pub fn can_unfreeze(&self, amount: Decimal) -> bool {
        self.frozen >= amount
    }
}
