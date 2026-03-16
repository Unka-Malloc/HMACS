use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, ComputeResourceId, ParticipantId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The three pricing modes for compute resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PricingMode {
    /// Fixed total price for a bulk package (e.g., 1000 GPU-hours at 500 USDC)
    BulkPackage {
        total_units: Decimal,
        unit_label: String,
        total_price: Decimal,
        asset: AssetSymbol,
    },
    /// Per-unit-time pricing (e.g., 0.50 USDC per GPU-hour)
    PerUnitTime {
        price_per_unit: Decimal,
        unit_label: String,
        asset: AssetSymbol,
        min_units: Option<Decimal>,
        max_units: Option<Decimal>,
    },
    /// Free pricing set by the seller, with platform guidance
    FreePrice {
        asking_price: Decimal,
        asset: AssetSymbol,
        unit_label: String,
        quantity: Decimal,
    },
}

impl PricingMode {
    pub fn asset(&self) -> AssetSymbol {
        match self {
            Self::BulkPackage { asset, .. } => *asset,
            Self::PerUnitTime { asset, .. } => *asset,
            Self::FreePrice { asset, .. } => *asset,
        }
    }

    pub fn effective_price_per_unit(&self) -> Option<Decimal> {
        match self {
            Self::BulkPackage {
                total_units,
                total_price,
                ..
            } => {
                if total_units.is_zero() {
                    None
                } else {
                    Some(*total_price / *total_units)
                }
            }
            Self::PerUnitTime {
                price_per_unit, ..
            } => Some(*price_per_unit),
            Self::FreePrice {
                asking_price,
                quantity,
                ..
            } => {
                if quantity.is_zero() {
                    None
                } else {
                    Some(*asking_price / *quantity)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeListing {
    pub id: uuid::Uuid,
    pub resource_id: ComputeResourceId,
    pub seller_id: ParticipantId,
    pub pricing: PricingMode,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListing {
    pub resource_id: ComputeResourceId,
    pub pricing: PricingMode,
}

/// Price guidance provided by the platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceGuidance {
    pub unit_label: String,
    pub asset: AssetSymbol,
    pub average_price: Decimal,
    pub median_price: Decimal,
    pub low_price: Decimal,
    pub high_price: Decimal,
    pub recommended_range: (Decimal, Decimal),
    pub supply_count: u64,
    pub demand_count: u64,
    pub supply_demand_ratio: f64,
    pub computed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bulk_price_per_unit() {
        let mode = PricingMode::BulkPackage {
            total_units: dec!(1000),
            unit_label: "GPU-hour".into(),
            total_price: dec!(500),
            asset: AssetSymbol::Usdc,
        };
        assert_eq!(mode.effective_price_per_unit(), Some(dec!(0.5)));
    }

    #[test]
    fn test_per_unit_time_price() {
        let mode = PricingMode::PerUnitTime {
            price_per_unit: dec!(0.75),
            unit_label: "GPU-hour".into(),
            asset: AssetSymbol::Usdc,
            min_units: Some(dec!(1)),
            max_units: None,
        };
        assert_eq!(mode.effective_price_per_unit(), Some(dec!(0.75)));
    }
}
