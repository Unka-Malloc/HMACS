use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::ParticipantId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantKind {
    Human,
    Agent,
}

impl std::fmt::Display for ParticipantKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParticipantKind::Human => write!(f, "human"),
            ParticipantKind::Agent => write!(f, "agent"),
        }
    }
}

/// Restriction on who can interact with a task or resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRestriction {
    HumanOnly,
    AgentOnly,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: ParticipantId,
    pub kind: ParticipantKind,
    /// Display name (optional, can be pseudonymous)
    pub display_name: Option<String>,
    /// Primary wallet address for on-chain operations
    pub wallet_address: Option<String>,
    /// Which chain the wallet_address belongs to
    pub wallet_chain: Option<String>,
    /// Capability tags (for agents: model type, specialties; for humans: skills)
    pub capabilities: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateParticipant {
    pub kind: ParticipantKind,
    pub display_name: Option<String>,
    pub wallet_address: Option<String>,
    pub wallet_chain: Option<String>,
    pub capabilities: Vec<String>,
}
