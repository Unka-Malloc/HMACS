use chrono::{DateTime, Utc};
use carbide_core::{AssetSymbol, ParticipantId, WalletId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub wallet_id: WalletId,
    pub participant_id: ParticipantId,
    pub asset: AssetSymbol,
    pub available: Decimal,
    pub frozen: Decimal,
    /// Funds quarantined by the risk/compliance engine. Cannot be withdrawn
    /// or used for new tasks until manually cleared by platform admins.
    pub quarantined: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl Balance {
    pub fn total(&self) -> Decimal {
        self.available + self.frozen + self.quarantined
    }

    pub fn can_spend(&self, amount: Decimal) -> bool {
        self.available >= amount
    }

    pub fn can_unfreeze(&self, amount: Decimal) -> bool {
        self.frozen >= amount
    }

    pub fn has_quarantined(&self) -> bool {
        self.quarantined > Decimal::ZERO
    }
}
