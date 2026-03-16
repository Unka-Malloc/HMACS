use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, ComputeLeaseId, ComputeResourceId, ParticipantId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
    Expired,
}

/// A lease representing active usage of a compute resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeLease {
    pub id: ComputeLeaseId,
    pub resource_id: ComputeResourceId,
    pub listing_id: uuid::Uuid,
    pub buyer_id: ParticipantId,
    pub seller_id: ParticipantId,
    pub asset: AssetSymbol,
    pub price_per_unit: Decimal,
    pub unit_label: String,
    pub units_consumed: Decimal,
    pub max_units: Option<Decimal>,
    pub total_cost: Decimal,
    pub status: LeaseStatus,
    pub started_at: DateTime<Utc>,
    pub last_metered_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// A single metering event reported by the provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteringEvent {
    pub lease_id: ComputeLeaseId,
    pub units: Decimal,
    pub recorded_at: DateTime<Utc>,
}

impl ComputeLease {
    pub fn record_usage(&mut self, units: Decimal) {
        self.units_consumed += units;
        self.total_cost = self.units_consumed * self.price_per_unit;
        self.last_metered_at = Utc::now();

        if let Some(max) = self.max_units {
            if self.units_consumed >= max {
                self.status = LeaseStatus::Completed;
                self.ended_at = Some(Utc::now());
            }
        }
    }

    pub fn remaining_units(&self) -> Option<Decimal> {
        self.max_units
            .map(|max| (max - self.units_consumed).max(Decimal::ZERO))
    }

    pub fn complete(&mut self) {
        self.status = LeaseStatus::Completed;
        self.ended_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = LeaseStatus::Cancelled;
        self.ended_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmacs_core::ComputeLeaseId;
    use rust_decimal_macros::dec;

    fn make_lease() -> ComputeLease {
        ComputeLease {
            id: ComputeLeaseId::new(),
            resource_id: ComputeResourceId::new(),
            listing_id: uuid::Uuid::new_v4(),
            buyer_id: ParticipantId::new(),
            seller_id: ParticipantId::new(),
            asset: AssetSymbol::Usdc,
            price_per_unit: dec!(0.50),
            unit_label: "GPU-hour".into(),
            units_consumed: dec!(0),
            max_units: Some(dec!(100)),
            total_cost: dec!(0),
            status: LeaseStatus::Active,
            started_at: Utc::now(),
            last_metered_at: Utc::now(),
            ended_at: None,
        }
    }

    #[test]
    fn test_metering() {
        let mut lease = make_lease();
        lease.record_usage(dec!(10));
        assert_eq!(lease.units_consumed, dec!(10));
        assert_eq!(lease.total_cost, dec!(5));
        assert_eq!(lease.remaining_units(), Some(dec!(90)));
    }

    #[test]
    fn test_auto_complete_on_max() {
        let mut lease = make_lease();
        lease.record_usage(dec!(100));
        assert_eq!(lease.status, LeaseStatus::Completed);
        assert!(lease.ended_at.is_some());
    }
}
