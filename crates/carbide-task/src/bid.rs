use chrono::{DateTime, Utc};
use carbide_core::{AssetSymbol, BidId, ParticipantId, TaskId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BidStatus {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    pub id: BidId,
    pub task_id: TaskId,
    pub bidder_id: ParticipantId,
    pub amount: Decimal,
    pub asset: AssetSymbol,
    pub proposal: String,
    pub estimated_duration_hours: Option<f64>,
    pub status: BidStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBid {
    pub task_id: TaskId,
    pub amount: Decimal,
    pub asset: AssetSymbol,
    pub proposal: String,
    pub estimated_duration_hours: Option<f64>,
}
