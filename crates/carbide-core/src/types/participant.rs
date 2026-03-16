use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::{DelegatedKeyId, ParticipantId};

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

/// Actions an Agent's delegated key is permitted to sign.
/// Per the master-slave model, agents can ONLY accept tasks and submit results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPermission {
    AcceptTask,
    SubmitResult,
}

impl std::fmt::Display for AgentPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptTask => write!(f, "accept_task"),
            Self::SubmitResult => write!(f, "submit_result"),
        }
    }
}

/// A delegated session key allowing an Agent to act on behalf of a Human master.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedKey {
    pub id: DelegatedKeyId,
    pub agent_id: ParticipantId,
    pub master_id: ParticipantId,
    /// Ed25519 public key bytes (base58 encoded)
    pub public_key: String,
    pub permissions: Vec<AgentPermission>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

impl DelegatedKey {
    pub fn is_valid(&self) -> bool {
        !self.revoked && Utc::now() < self.expires_at
    }

    pub fn has_permission(&self, perm: AgentPermission) -> bool {
        self.is_valid() && self.permissions.contains(&perm)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: ParticipantId,
    pub kind: ParticipantKind,
    /// Decentralized Identifier (e.g., did:carbide:<uuid>)
    pub did: String,
    /// Display name (optional, can be pseudonymous)
    pub display_name: Option<String>,
    /// Primary wallet address for on-chain operations
    pub wallet_address: Option<String>,
    /// Which chain the wallet_address belongs to
    pub wallet_chain: Option<String>,
    /// For Agents: the Human master who controls fund rights.
    /// Humans have this set to None.
    pub master_id: Option<ParticipantId>,
    /// Whether this Human has a Soulbound Token (SBT) representing KYC.
    /// Only Humans can hold SBTs; Agents inherit compliance via their master.
    pub sbt_verified: bool,
    /// Capability tags (for agents: model type, specialties; for humans: skills)
    pub capabilities: Vec<String>,
    /// Whether this participant has been blacklisted (e.g., MPC malicious actor)
    pub blacklisted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Participant {
    pub fn is_agent(&self) -> bool {
        self.kind == ParticipantKind::Agent
    }

    pub fn is_human(&self) -> bool {
        self.kind == ParticipantKind::Human
    }

    pub fn can_hold_funds(&self) -> bool {
        self.is_human() && self.sbt_verified
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateParticipant {
    pub kind: ParticipantKind,
    pub display_name: Option<String>,
    pub wallet_address: Option<String>,
    pub wallet_chain: Option<String>,
    /// Required for Agent creation: the Human master's ID.
    pub master_id: Option<ParticipantId>,
    pub capabilities: Vec<String>,
}
