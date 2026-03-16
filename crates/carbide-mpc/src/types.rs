use chrono::{DateTime, Utc};
use carbide_core::{AssetSymbol, MpcTaskId, ParticipantId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::state::MpcTaskState;

/// A FROST threshold signature task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcTask {
    pub id: MpcTaskId,
    pub state: MpcTaskState,
    /// t-of-n threshold (e.g., t=3 of n=5)
    pub threshold: usize,
    pub total_agents: usize,
    /// The message/payload to be signed
    pub message: Vec<u8>,
    /// Agents that locked micro-stakes and entered the race
    pub participants: Vec<ParticipantId>,
    /// Agents that submitted valid shares
    pub valid_shares: Vec<(ParticipantId, SignatureShare)>,
    /// Agents identified as malicious (invalid VSS)
    pub malicious_agents: Vec<ParticipantId>,
    /// Micro-stake amount per agent (e.g., 0.1 USDC)
    pub stake_amount: Decimal,
    pub stake_asset: AssetSymbol,
    /// Bounty per honest agent that contributes to the final signature
    pub bounty_per_agent: Decimal,
    pub bounty_asset: AssetSymbol,
    /// The assembled final signature (set when Settled)
    pub final_signature: Option<Signature>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Retry count (incremented on each failure-retry cycle)
    pub retry_count: u32,
}

/// A single agent's signature share (Ed25519 FROST).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureShare {
    pub agent_id: ParticipantId,
    /// The share bytes (64 bytes for Ed25519)
    pub share_bytes: Vec<u8>,
    /// VSS commitment for verification
    pub vss_commitment: Vec<u8>,
    pub submitted_at: DateTime<Utc>,
}

/// The final assembled Ed25519 signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub bytes: Vec<u8>,
    pub assembled_at: DateTime<Utc>,
}

/// Errors specific to MPC operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MpcError {
    Timeout { task_id: MpcTaskId },
    InsufficientShares { have: usize, need: usize },
    InvalidShare { agent_id: ParticipantId },
    AssemblyFailed(String),
    TaskNotFound(MpcTaskId),
    InvalidState(String),
}

impl std::fmt::Display for MpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { task_id } => write!(f, "MPC task {} timed out", task_id),
            Self::InsufficientShares { have, need } => {
                write!(f, "Only {}/{} valid shares collected", have, need)
            }
            Self::InvalidShare { agent_id } => {
                write!(f, "Invalid share from agent {}", agent_id)
            }
            Self::AssemblyFailed(msg) => write!(f, "Signature assembly failed: {}", msg),
            Self::TaskNotFound(id) => write!(f, "MPC task {} not found", id),
            Self::InvalidState(msg) => write!(f, "Invalid MPC state: {}", msg),
        }
    }
}

impl From<MpcError> for carbide_core::CarbideError {
    fn from(err: MpcError) -> Self {
        carbide_core::CarbideError::MpcError(err.to_string())
    }
}
