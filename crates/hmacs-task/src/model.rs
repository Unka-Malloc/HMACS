use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, ParticipantId, ParticipantRestriction, TaskId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::state_machine::TaskStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub creator_id: ParticipantId,
    pub title: String,
    pub description: String,
    pub skill_tags: Vec<String>,
    pub budget_asset: AssetSymbol,
    pub budget_amount: Decimal,
    pub status: TaskStatus,
    pub restriction: ParticipantRestriction,
    pub assigned_to: Option<ParticipantId>,
    pub deadline: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deliverable_url: Option<String>,
    pub deliverable_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: String,
    pub skill_tags: Vec<String>,
    pub budget_asset: AssetSymbol,
    pub budget_amount: Decimal,
    pub restriction: ParticipantRestriction,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub skill_tags: Option<Vec<String>>,
    pub budget_amount: Option<Decimal>,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverTask {
    pub deliverable_url: Option<String>,
    pub deliverable_notes: Option<String>,
}
