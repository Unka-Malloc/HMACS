use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, OrderId, ParticipantId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    /// Fixed price, execute immediately if match exists
    Market,
    /// Specified price, sit in the book until matched
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub participant_id: ParticipantId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub asset: AssetSymbol,
    pub price: Decimal,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    /// Opaque reference back to the domain (task_id, compute_lease_id, etc.)
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
}

impl Order {
    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }

    pub fn is_fully_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrder {
    pub participant_id: ParticipantId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub asset: AssetSymbol,
    pub price: Decimal,
    pub quantity: Decimal,
    pub expires_at: Option<DateTime<Utc>>,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
}

/// Result of a match between two orders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub buy_order_id: OrderId,
    pub sell_order_id: OrderId,
    pub price: Decimal,
    pub quantity: Decimal,
    pub asset: AssetSymbol,
    pub executed_at: DateTime<Utc>,
}
